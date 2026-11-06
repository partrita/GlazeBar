//! UI-thread hooks only enqueue work. No IPC or waiting inside callbacks.
use anyhow::{Result, ensure};
use std::{
    cell::RefCell,
    ptr::null_mut,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::Sender,
    },
};
use windows_sys::Win32::{
    Foundation::*,
    System::Threading::*,
    UI::{Accessibility::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub enum Event {
    Control(&'static str),
    Foreground,
    PointerDown(POINT),
}

pub fn hotkey_command(id: usize, remote: bool) -> Option<&'static str> {
    if remote {
        return None;
    }
    match id {
        2 => Some("Bar"),
        3 => Some("Wallpaper"),
        4 => Some("Toggle"),
        5 => Some("Sheet"),
        _ => None,
    }
}

pub fn win_event_command(event: u32, remote: bool) -> Option<&'static str> {
    if !remote && event == EVENT_SYSTEM_SWITCHEND {
        Some("FocusResync")
    } else {
        None
    }
}

struct Context {
    tx: Sender<Event>,
    thread: u32,
    remote: Arc<AtomicBool>,
    intercept: bool,
    local_hotkeys: bool,
}
thread_local! { static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) }; }
impl Context {
    fn emit(&self, event: Event) {
        if self.tx.send(event).is_ok() {
            unsafe {
                PostThreadMessageW(self.thread, WM_APP + 2, 0, 0);
            }
        }
    }
    fn sync_local_hotkeys(&mut self, is_remote: bool) {
        if !self.intercept {
            return;
        }
        unsafe {
            if is_remote && self.local_hotkeys {
                UnregisterHotKey(null_mut(), 4);
                UnregisterHotKey(null_mut(), 5);
                self.local_hotkeys = false;
            } else if !is_remote && !self.local_hotkeys {
                let _ = RegisterHotKey(null_mut(), 4, MOD_NOREPEAT | MOD_ALT, 0x4d);
                let _ = RegisterHotKey(null_mut(), 5, MOD_NOREPEAT | MOD_WIN, 0x70);
                self.local_hotkeys = true;
            }
        }
    }
}

unsafe extern "system" fn win_event(
    _: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _: i32,
    _: i32,
    _: u32,
    _: u32,
) {
    CONTEXT.with(|slot| {
        if let Some(c) = slot.borrow_mut().as_mut() {
            match event {
                EVENT_SYSTEM_FOREGROUND => {
                    let now_remote = is_remote(hwnd);
                    c.remote.store(now_remote, Ordering::Relaxed);
                    c.sync_local_hotkeys(now_remote);
                    c.emit(Event::Foreground);
                }
                EVENT_SYSTEM_CAPTURESTART => {
                    let mut pt = POINT::default();
                    if unsafe { GetCursorPos(&mut pt) } != 0 {
                        c.emit(Event::PointerDown(pt));
                    }
                }
                _ => {
                    if let Some(cmd) = win_event_command(event, c.remote.load(Ordering::Relaxed)) {
                        c.emit(Event::Control(cmd));
                    }
                }
            }
        }
    });
}

fn remote_name(path: &str) -> bool {
    matches!(
        path.rsplit(['\\', '/'])
            .next()
            .unwrap_or("")
            .to_ascii_lowercase()
            .as_str(),
        "mstsc.exe" | "msrdc.exe" | "rdclientwpf.exe"
    )
}
fn is_remote(hwnd: HWND) -> bool {
    unsafe {
        let mut pid = 0;
        GetWindowThreadProcessId(hwnd, &mut pid);
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return false;
        }
        let mut buf = [0u16; 32768];
        let mut len = buf.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buf.as_mut_ptr(), &mut len);
        CloseHandle(process);
        ok != 0 && remote_name(&String::from_utf16_lossy(&buf[..len as usize]))
    }
}
pub struct Hooks {
    foreground: HWINEVENTHOOK,
    hotkeys: bool,
}
impl Hooks {
    pub fn install(tx: Sender<Event>, remote: Arc<AtomicBool>, intercept: bool) -> Result<Self> {
        ensure!(
            CONTEXT.with(|c| c.borrow().is_none()),
            "Hooks already installed on this thread"
        );
        unsafe {
            let initial_remote = is_remote(GetForegroundWindow());
            remote.store(initial_remote, Ordering::Relaxed);
            CONTEXT.with(|slot| {
                let mut ctx = Context {
                    tx,
                    thread: GetCurrentThreadId(),
                    remote,
                    intercept,
                    local_hotkeys: false,
                };
                ctx.sync_local_hotkeys(initial_remote);
                *slot.borrow_mut() = Some(ctx);
            });
            let mut hooks = Self {
                foreground: null_mut(),
                hotkeys: intercept,
            };
            hooks.foreground = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_SWITCHEND,
                null_mut(),
                Some(win_event),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
            ensure!(
                !hooks.foreground.is_null(),
                "Cannot install foreground hook: {}",
                GetLastError()
            );
            if intercept {
                ensure!(
                    RegisterHotKey(
                        null_mut(),
                        2,
                        MOD_NOREPEAT | MOD_WIN | MOD_ALT | MOD_SHIFT,
                        0x42
                    ) != 0,
                    "Cannot register Win+Alt+Shift+B bar hotkey: {}",
                    GetLastError()
                );
                ensure!(
                    RegisterHotKey(
                        null_mut(),
                        3,
                        MOD_NOREPEAT | MOD_WIN | MOD_CONTROL | MOD_ALT,
                        0x20
                    ) != 0,
                    "Cannot register wallpaper hotkey"
                );
            }
            Ok(hooks)
        }
    }
}
impl Drop for Hooks {
    fn drop(&mut self) {
        unsafe {
            if !self.foreground.is_null() {
                UnhookWinEvent(self.foreground);
            }
            if self.hotkeys {
                UnregisterHotKey(null_mut(), 2);
                UnregisterHotKey(null_mut(), 3);
                UnregisterHotKey(null_mut(), 4);
                UnregisterHotKey(null_mut(), 5);
            }
            CONTEXT.with(|slot| *slot.borrow_mut() = None);
        }
    }
}
pub fn pause_transition(remote: bool, paused: bool, owned: bool) -> (bool, bool) {
    if remote && !paused {
        (true, true)
    } else if !remote && owned {
        (paused, false)
    } else {
        (false, owned)
    }
}
pub fn remote_covers(bounds: crate::graphics::Rect) -> bool {
    if !CONTEXT.with(|c| {
        c.borrow()
            .as_ref()
            .is_some_and(|c| c.remote.load(Ordering::Relaxed))
    }) {
        return false;
    }
    unsafe {
        let mut r = RECT::default();
        GetWindowRect(GetForegroundWindow(), &mut r) != 0
            && r.left <= bounds.x + 8
            && r.right >= bounds.x + bounds.w - 8
            && r.top <= bounds.y + 8
            && r.bottom >= bounds.y + bounds.h
    }
}
pub fn validate() -> Result<()> {
    let (tx, _rx) = std::sync::mpsc::channel();
    drop(Hooks::install(tx, Arc::new(AtomicBool::new(false)), false)?);
    ensure!(
        CONTEXT.with(|c| c.borrow().is_none()),
        "Hook context leaked"
    );
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hotkeys_map_to_expected_commands_when_local() {
        assert_eq!(hotkey_command(2, false), Some("Bar"));
        assert_eq!(hotkey_command(3, false), Some("Wallpaper"));
        assert_eq!(hotkey_command(4, false), Some("Toggle"));
        assert_eq!(hotkey_command(5, false), Some("Sheet"));
        assert_eq!(hotkey_command(99, false), None);
    }
    #[test]
    fn remote_blocks_local_hotkey_commands() {
        assert_eq!(hotkey_command(2, true), None);
        assert_eq!(hotkey_command(3, true), None);
        assert_eq!(hotkey_command(4, true), None);
        assert_eq!(hotkey_command(5, true), None);
    }
    #[test]
    fn switch_end_win_event_resyncs_only_when_local() {
        assert_eq!(
            win_event_command(EVENT_SYSTEM_SWITCHEND, false),
            Some("FocusResync")
        );
        assert_eq!(win_event_command(EVENT_SYSTEM_SWITCHEND, true), None);
        assert_eq!(win_event_command(EVENT_SYSTEM_FOREGROUND, false), None);
    }
    #[test]
    fn only_undo_pause_owned_by_us() {
        assert_eq!(pause_transition(true, false, false), (true, true));
        assert_eq!(pause_transition(true, true, false), (false, false));
        assert_eq!(pause_transition(false, true, false), (false, false));
        assert_eq!(pause_transition(false, true, true), (true, false));
        assert_eq!(pause_transition(false, false, true), (false, false));
    }
    #[test]
    fn remote_process_names_are_exact() {
        assert!(remote_name("C:\\Windows\\System32\\MSTSC.EXE"));
        assert!(!remote_name("C:\\mstsc.exe.backup"));
    }
}
