use anyhow::{Result, ensure};
use std::ffi::c_void;
use windows_sys::core::GUID;

#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(dead_code)]
pub enum MediaCommand {
    PlayPause,
    Next,
    Prev,
}

#[cfg(windows)]
#[allow(non_snake_case, dead_code)]
mod imp {
    use super::*;
    use std::ptr::{null, null_mut};
    use windows::Win32::System::Com::*;
    use windows::core::{HRESULT, Interface};
    use windows_sys::Win32::{Foundation::*, System::Power::*, UI::WindowsAndMessaging::*};

    const CLSID_MMDEVICE_ENUMERATOR: GUID = GUID::from_u128(0xbcde0395_e52f_467c_8e3d_c4579291692e);
    const IID_IMMDEVICE_ENUMERATOR: GUID = GUID::from_u128(0xa95664d2_9614_4f35_a746_de8db63617e6);
    const IID_IAUDIO_ENDPOINT_VOLUME: GUID =
        GUID::from_u128(0x5cdf2c82_841e_4546_9722_0cf74078229a);

    #[repr(C)]
    struct IUnknownVtbl {
        pub QueryInterface: unsafe extern "system" fn(
            this: *mut c_void,
            riid: *const GUID,
            ppv: *mut *mut c_void,
        ) -> HRESULT,
        pub AddRef: unsafe extern "system" fn(this: *mut c_void) -> u32,
        pub Release: unsafe extern "system" fn(this: *mut c_void) -> u32,
    }

    #[repr(C)]
    struct IMMDeviceEnumeratorVtbl {
        pub parent: IUnknownVtbl,
        pub EnumAudioEndpoints: unsafe extern "system" fn(
            this: *mut c_void,
            data_flow: i32,
            dw_state_mask: u32,
            pp_devices: *mut *mut c_void,
        ) -> HRESULT,
        pub GetDefaultAudioEndpoint: unsafe extern "system" fn(
            this: *mut c_void,
            data_flow: i32,
            role: i32,
            pp_endpoint: *mut *mut c_void,
        ) -> HRESULT,
    }

    #[repr(C)]
    struct IMMDeviceVtbl {
        pub parent: IUnknownVtbl,
        pub Activate: unsafe extern "system" fn(
            this: *mut c_void,
            iid: *const GUID,
            dw_cls_ctx: u32,
            p_activation_params: *const c_void,
            pp_interface: *mut *mut c_void,
        ) -> HRESULT,
    }

    #[repr(C)]
    struct IAudioEndpointVolumeVtbl {
        pub parent: IUnknownVtbl,
        pub RegisterControlChangeNotify:
            unsafe extern "system" fn(this: *mut c_void, p_notify: *mut c_void) -> HRESULT,
        pub UnregisterControlChangeNotify:
            unsafe extern "system" fn(this: *mut c_void, p_notify: *mut c_void) -> HRESULT,
        pub GetChannelCount:
            unsafe extern "system" fn(this: *mut c_void, pn_channel_count: *mut u32) -> HRESULT,
        pub SetMasterVolumeLevel: unsafe extern "system" fn(
            this: *mut c_void,
            f_level_db: f32,
            pguid_event_context: *const GUID,
        ) -> HRESULT,
        pub SetMasterVolumeLevelScalar: unsafe extern "system" fn(
            this: *mut c_void,
            f_level: f32,
            pguid_event_context: *const GUID,
        ) -> HRESULT,
        pub GetMasterVolumeLevel:
            unsafe extern "system" fn(this: *mut c_void, pf_level_db: *mut f32) -> HRESULT,
        pub GetMasterVolumeLevelScalar:
            unsafe extern "system" fn(this: *mut c_void, pf_level: *mut f32) -> HRESULT,
        pub SetChannelVolumeLevel: unsafe extern "system" fn(
            this: *mut c_void,
            n_channel: u32,
            f_level_db: f32,
            pguid_event_context: *const GUID,
        ) -> HRESULT,
        pub SetChannelVolumeLevelScalar: unsafe extern "system" fn(
            this: *mut c_void,
            n_channel: u32,
            f_level: f32,
            pguid_event_context: *const GUID,
        ) -> HRESULT,
        pub GetChannelVolumeLevel: unsafe extern "system" fn(
            this: *mut c_void,
            n_channel: u32,
            pf_level_db: *mut f32,
        ) -> HRESULT,
        pub GetChannelVolumeLevelScalar: unsafe extern "system" fn(
            this: *mut c_void,
            n_channel: u32,
            pf_level: *mut f32,
        ) -> HRESULT,
        pub SetMute: unsafe extern "system" fn(
            this: *mut c_void,
            b_mute: i32,
            pguid_event_context: *const GUID,
        ) -> HRESULT,
        pub GetMute: unsafe extern "system" fn(this: *mut c_void, pb_mute: *mut i32) -> HRESULT,
    }

    struct AudioEndpointVolumeWrapper(*mut c_void);

    impl AudioEndpointVolumeWrapper {
        fn new() -> Result<Self> {
            unsafe {
                let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
                let hr = CoCreateInstance(
                    &std::mem::transmute(CLSID_MMDEVICE_ENUMERATOR),
                    None,
                    CLSCTX_ALL,
                );
                let enumerator: windows::core::IUnknown = hr?;
                let enumerator_raw = enumerator.as_raw();

                let enum_vtbl = *(enumerator_raw as *mut *mut IMMDeviceEnumeratorVtbl);
                let mut device_ptr: *mut c_void = null_mut();
                // dataFlow = 0 (eRender), role = 1 (eMultimedia)
                let hr_device =
                    ((*enum_vtbl).GetDefaultAudioEndpoint)(enumerator_raw, 0, 1, &mut device_ptr);
                ensure!(
                    hr_device.0 >= 0,
                    "GetDefaultAudioEndpoint failed: {hr_device:?}"
                );

                let dev_vtbl = *(device_ptr as *mut *mut IMMDeviceVtbl);
                let mut volume_ptr: *mut c_void = null_mut();
                let hr_vol = ((*dev_vtbl).Activate)(
                    device_ptr,
                    &IID_IAUDIO_ENDPOINT_VOLUME,
                    CLSCTX_ALL.0,
                    null(),
                    &mut volume_ptr,
                );
                // Release device
                let dev_unknown = *(device_ptr as *mut *mut IUnknownVtbl);
                ((*dev_unknown).Release)(device_ptr);

                ensure!(
                    hr_vol.0 >= 0,
                    "Activate IAudioEndpointVolume failed: {hr_vol:?}"
                );
                Ok(Self(volume_ptr))
            }
        }

        fn get_volume_scalar(&self) -> Result<f32> {
            unsafe {
                let vtbl = *(self.0 as *mut *mut IAudioEndpointVolumeVtbl);
                let mut level: f32 = 0.0;
                let hr = ((*vtbl).GetMasterVolumeLevelScalar)(self.0, &mut level);
                ensure!(hr.0 >= 0, "GetMasterVolumeLevelScalar failed");
                Ok(level)
            }
        }

        fn set_volume_scalar(&self, level: f32) -> Result<()> {
            unsafe {
                let vtbl = *(self.0 as *mut *mut IAudioEndpointVolumeVtbl);
                let hr =
                    ((*vtbl).SetMasterVolumeLevelScalar)(self.0, level.clamp(0.0, 1.0), null());
                ensure!(hr.0 >= 0, "SetMasterVolumeLevelScalar failed");
                Ok(())
            }
        }

        fn get_mute(&self) -> Result<bool> {
            unsafe {
                let vtbl = *(self.0 as *mut *mut IAudioEndpointVolumeVtbl);
                let mut mute: i32 = 0;
                let hr = ((*vtbl).GetMute)(self.0, &mut mute);
                ensure!(hr.0 >= 0, "GetMute failed");
                Ok(mute != 0)
            }
        }

        fn set_mute(&self, mute: bool) -> Result<()> {
            unsafe {
                let vtbl = *(self.0 as *mut *mut IAudioEndpointVolumeVtbl);
                let hr = ((*vtbl).SetMute)(self.0, if mute { 1 } else { 0 }, null());
                ensure!(hr.0 >= 0, "SetMute failed");
                Ok(())
            }
        }
    }

    impl Drop for AudioEndpointVolumeWrapper {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let unknown = *(self.0 as *mut *mut IUnknownVtbl);
                    ((*unknown).Release)(self.0);
                }
            }
        }
    }

    pub fn get_volume_and_mute() -> (u8, bool) {
        match AudioEndpointVolumeWrapper::new() {
            Ok(vol) => {
                let level = vol.get_volume_scalar().unwrap_or(0.5);
                let mute = vol.get_mute().unwrap_or(false);
                ((level * 100.0).round().clamp(0.0, 100.0) as u8, mute)
            }
            Err(_) => (50, false),
        }
    }

    pub fn set_volume(vol: u8) -> Result<()> {
        let wrapper = AudioEndpointVolumeWrapper::new()?;
        wrapper.set_volume_scalar((vol as f32) / 100.0)?;
        if vol > 0 && wrapper.get_mute().unwrap_or(false) {
            let _ = wrapper.set_mute(false);
        }
        Ok(())
    }

    pub fn toggle_mute() -> Result<bool> {
        let wrapper = AudioEndpointVolumeWrapper::new()?;
        let muted = wrapper.get_mute().unwrap_or(false);
        wrapper.set_mute(!muted)?;
        Ok(!muted)
    }

    pub fn send_media_command(cmd: MediaCommand) {
        unsafe {
            let app_cmd = match cmd {
                MediaCommand::PlayPause => 14, // APPCOMMAND_MEDIA_PLAY_PAUSE
                MediaCommand::Next => 11,      // APPCOMMAND_MEDIA_NEXTTRACK
                MediaCommand::Prev => 12,      // APPCOMMAND_MEDIA_PREVIOUSTRACK
            };
            let shell = GetShellWindow();
            if !shell.is_null() {
                SendMessageW(
                    shell,
                    WM_APPCOMMAND,
                    shell as usize,
                    ((app_cmd as usize) << 16) as isize,
                );
            }
        }
    }

    pub fn set_caffeine(enabled: bool) {
        unsafe {
            if enabled {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED | ES_DISPLAY_REQUIRED);
            } else {
                SetThreadExecutionState(ES_CONTINUOUS);
            }
        }
    }

    struct EnumMediaData {
        title: String,
        is_playing: bool,
    }

    unsafe extern "system" fn enum_windows_callback(hwnd: HWND, lparam: LPARAM) -> i32 {
        unsafe {
            let data = &mut *(lparam as *mut EnumMediaData);
            if IsWindowVisible(hwnd) == 0 {
                return 1;
            }

            let mut buffer = [0u16; 512];
            let len = GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
            if len > 0 {
                let title = String::from_utf16_lossy(&buffer[..len as usize]);
                let lower = title.to_lowercase();

                // Check Spotify
                if title.contains(" - ")
                    && !lower.ends_with(" - google chrome")
                    && !lower.ends_with(" - edge")
                    && !lower.ends_with(" - visual studio code")
                {
                    // Likely a music track (e.g., "Artist - Title")
                    data.title = title;
                    data.is_playing = true;
                    return 0; // stop enumeration
                }

                // Check YouTube in browser
                if lower.contains(" - youtube") {
                    let clean = title.split(" - YouTube").next().unwrap_or(&title);
                    data.title = clean.trim().to_string();
                    data.is_playing = true;
                    return 0;
                }
            }
            1
        }
    }

    pub fn detect_media() -> (String, bool) {
        unsafe {
            let mut data = EnumMediaData {
                title: String::new(),
                is_playing: false,
            };
            EnumWindows(
                Some(enum_windows_callback),
                (&mut data as *mut EnumMediaData) as LPARAM,
            );
            (data.title, data.is_playing)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::*;

    pub fn get_volume_and_mute() -> (u8, bool) {
        (75, false)
    }

    pub fn set_volume(_vol: u8) -> Result<()> {
        Ok(())
    }

    pub fn toggle_mute() -> Result<bool> {
        Ok(false)
    }

    pub fn send_media_command(_cmd: MediaCommand) {}

    pub fn set_caffeine(_enabled: bool) {}

    pub fn detect_media() -> (String, bool) {
        (String::new(), false)
    }
}

pub use imp::*;
