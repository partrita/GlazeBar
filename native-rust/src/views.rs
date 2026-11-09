use crate::{
    graphics::{Canvas, Rect},
    ipc::Monitor,
    sensors::Sensors,
    settings_ui::BarSettings,
};
use chrono::{Datelike, Timelike};
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum Action {
    Command(String),
    Scale,
    Power,
    PowerSelect(String),
    Wifi,
    WifiScan(bool),
    WifiSelect(usize),
    WifiConnect,
    WifiSettings,
    WifiScroll(i32),
    Brightness,
    Battery,
    Close,
    Sheet,
    Wallpaper,
    OpenSettings,
    Settings(crate::settings_ui::Edit),
    PrecacheStop,
    Online(crate::online_ui::Edit),
    Compare(crate::style_compare_ui::Edit),
    WindowsOption(crate::windows_options_ui::Edit),
    Picker(crate::wallpaper_ui::Action),
    HubTab(crate::hub::Tab),
    HubRetryTheme,
    QuickTheme,
    QuickThemeSelect(String),
    QuickThemeLight(bool),
    VolumeMute,
    VolumeSet(u8),
    MediaPlayPause,
    MediaNext,
    MediaPrev,
    CaffeineToggle,
}
#[derive(Clone, Debug)]
pub struct Hit {
    pub rect: Rect,
    pub action: Action,
}

#[cfg(test)]
mod wifi_visibility_tests {
    use super::*;

    #[test]
    fn missing_wifi_has_no_hit_target_but_disconnected_wifi_does() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let mut wifi_positions = Vec::new();
        for (available, ssid) in [(false, ""), (true, ""), (true, "Sample Wi-Fi"), (false, "")] {
            let sensors = Sensors {
                wifi_available: available,
                network: ssid.into(),
                ..Default::default()
            };
            let mut hits = Vec::new();
            crate::graphics::render_png(
                root,
                &output.path().join("bar.png"),
                1645,
                40,
                1.,
                crate::graphics::Palette::load(root),
                |canvas| {
                    hits = bar(
                        canvas,
                        &Monitor::default(),
                        &sensors,
                        false,
                        false,
                        false,
                        "12:00",
                        None,
                        None,
                    );
                },
            )
            .unwrap();
            assert_eq!(hits.iter().any(|hit| hit.action == Action::Wifi), available);
            wifi_positions.push(
                hits.iter()
                    .find(|hit| hit.action == Action::Command("toggle-tiling-direction".into()))
                    .unwrap()
                    .rect
                    .x,
            );
        }
        // The missing item releases its layout space, including after removal.
        assert!(wifi_positions[0] > wifi_positions[1]);
        assert_eq!(wifi_positions[0], wifi_positions[3]);
    }
}
fn hit(hits: &mut Vec<Hit>, rect: Rect, action: Action) {
    hits.push(Hit { rect, action });
}
fn metric(
    c: &mut Canvas,
    right: &mut i32,
    hits: &mut Vec<Hit>,
    icon: &str,
    text: &str,
    color: u32,
    action: Option<Action>,
) -> Rect {
    let width = c.p(24.) + c.measure(text, 12.5, false);
    let r = Rect::new(*right - width, 0, width, c.height);
    c.icon(
        icon,
        Rect::new(r.x, (c.height - c.p(17.)) / 2, c.p(17.), c.p(17.)),
        color,
    );
    c.text(
        text,
        Rect::new(r.x + c.p(24.), 0, width - c.p(24.), c.height),
        color,
        12.5,
        false,
        2,
    );
    if let Some(action) = action {
        hit(hits, r, action)
    }
    *right = r.x - c.p(18.);
    r
}
pub fn bar(
    c: &mut Canvas,
    m: &Monitor,
    s: &Sensors,
    paused: bool,
    resize: bool,
    vertical: bool,
    clock: &str,
    brightness: Option<u8>,
    weather: Option<(&str, &str)>,
) -> Vec<Hit> {
    bar_with_settings(
        c,
        m,
        s,
        paused,
        resize,
        vertical,
        clock,
        brightness,
        weather,
        &BarSettings::default(),
    )
}
pub fn bar_with_settings(
    c: &mut Canvas,
    m: &Monitor,
    s: &Sensors,
    paused: bool,
    resize: bool,
    vertical: bool,
    clock: &str,
    brightness: Option<u8>,
    weather: Option<(&str, &str)>,
    settings: &BarSettings,
) -> Vec<Hit> {
    let mut hits = vec![];
    let is_transparent = settings.is_transparent();
    let saved_ui = c.ui;
    if is_transparent {
        c.ui.text = 0x00FFFFFF;
        c.ui.muted = 0x00FFFFFF;
        c.ui.border = 0x00FFFFFF;
        c.ui.accent = 0x00FFFFFF;
        c.ui.surface = c.ui.bg;
    }
    let ui = c.ui;
    let h = c.height;
    let pad_y = c.p(4.).max((h - c.p(24.)) / 2);
    let ws_h = h - pad_y * 2;
    let mut left = c.p(18.);
    let logo_rect = Rect::new(left - c.p(4.), (h - c.p(24.)) / 2, c.p(24.), c.p(24.));
    for x in 0..2 {
        for y in 0..2 {
            c.fill(
                Rect::new(
                    left + x * c.p(8.),
                    (h - c.p(14.)) / 2 + y * c.p(8.),
                    c.p(6.),
                    c.p(6.),
                ),
                if is_transparent { 0x00FFFFFF } else { ui.muted },
            )
        }
    }
    hit(&mut hits, logo_rect, Action::OpenSettings);
    left += c.p(34.);
    for ws in &m.workspaces {
        let width = c.p(28.).max(c.measure(&ws.display, 12.5, true) + c.p(16.));
        let r = Rect::new(left, pad_y, width, ws_h);
        if is_transparent {
            let is_active = ws.focused || ws.displayed;
            if is_active {
                c.frame(r, 0x00FFFFFF);
                let inner = Rect::new(
                    r.x + 1,
                    r.y + 1,
                    r.w.saturating_sub(2),
                    r.h.saturating_sub(2),
                );
                c.frame(inner, 0x00FFFFFF);
            } else {
                c.frame(r, 0x00888888);
            }
            c.text(&ws.display, r, 0x00FFFFFF, 12.5, true, 1);
        } else {
            c.fill(
                r,
                if ws.focused || ws.displayed {
                    ui.selected
                } else {
                    ui.surface
                },
            );
            c.text(&ws.display, r, ui.text, 12.5, true, 1);
        }
        hit(
            &mut hits,
            r,
            Action::Command(format!("focus --workspace {}", ws.name)),
        );
        left += width + c.p(4.);
    }
    if settings.show_quick_buttons {
        left += c.p(6.);
        for (label, action) in [
            ("테마", Action::QuickTheme),
            ("설정", Action::OpenSettings),
            ("단축키", Action::Sheet),
        ] {
            let width = c.measure(label, 11.5, false) + c.p(16.);
            let r = Rect::new(left, pad_y, width, ws_h);
            if is_transparent {
                c.frame(r, 0x00888888);
                c.text(label, r, 0x00FFFFFF, 11.5, false, 1);
            } else {
                c.fill(r, ui.surface);
                c.text(label, r, ui.muted, 11.5, false, 1);
            }
            hit(&mut hits, r, action);
            left += width + c.p(4.);
        }
    }
    let mut right = c.width - c.p(18.);
    let order = settings.normalized_widget_order();

    for widget_id in order.iter() {
        match widget_id.as_str() {
            "weather" if settings.show_weather => {
                if let Some((icon, text)) = weather {
                    metric(c, &mut right, &mut hits, icon, text, ui.muted, None);
                }
            }
            "battery" if settings.show_battery && s.battery => {
                let mut vw = 0;
                for d in 0..10 {
                    for sign in ["+", "−"] {
                        vw = vw.max(c.measure(&format!("{sign}{d}{d}.{d}"), 12.5, true));
                    }
                }
                let uw = c.measure("W", 12.5, false);
                let r = Rect::new(right - c.p(26.) - vw - uw, 0, c.p(26.) + vw + uw, h);
                let color = if s.charging {
                    ui.success
                } else if s.discharging {
                    ui.warning
                } else {
                    ui.muted
                };
                let value = if s.known {
                    format!(
                        "{}{:.1}",
                        if s.charging {
                            "+"
                        } else if s.discharging {
                            "−"
                        } else {
                            ""
                        },
                        s.watts
                    )
                } else {
                    "—".into()
                };
                c.icon(
                    "zap",
                    Rect::new(r.x, (h - c.p(15.)) / 2, c.p(15.), c.p(15.)),
                    color,
                );
                c.text(
                    &value,
                    Rect::new(r.x + c.p(22.), 0, vw, h),
                    color,
                    12.5,
                    true,
                    2,
                );
                c.text(
                    "W",
                    Rect::new(r.x + c.p(26.) + vw, 0, uw, h),
                    color,
                    12.5,
                    false,
                    2,
                );
                hit(&mut hits, r, Action::Battery);
                right = r.x - c.p(18.);
                let icon = if s.percent >= 65. {
                    "battery-full"
                } else if s.percent >= 30. {
                    "battery-medium"
                } else if s.percent > 15. {
                    "battery-low"
                } else {
                    "battery"
                };
                metric(
                    c,
                    &mut right,
                    &mut hits,
                    icon,
                    &format!("{:.0}%", s.percent),
                    ui.text,
                    Some(Action::Power),
                );
            }
            "cpu" if settings.show_cpu => {
                metric(
                    c,
                    &mut right,
                    &mut hits,
                    "cpu",
                    &format!("{:.0}%", s.cpu),
                    ui.text,
                    None,
                );
            }
            "memory" if settings.show_memory => {
                metric(
                    c,
                    &mut right,
                    &mut hits,
                    "memory-stick",
                    &format!("{:.0}%", s.memory),
                    ui.text,
                    None,
                );
            }
            "wifi" if settings.show_wifi && s.wifi_available => {
                let network = if s.network.is_empty() {
                    "Wi-Fi".to_string()
                } else if s.network.chars().count() > 24 {
                    format!("{}…", s.network.chars().take(23).collect::<String>())
                } else {
                    s.network.clone()
                };
                metric(
                    c,
                    &mut right,
                    &mut hits,
                    "wifi",
                    &network,
                    ui.muted,
                    Some(Action::Wifi),
                );
            }
            "volume" if settings.show_volume => {
                let (icon, text, color) = if s.muted {
                    ("volume-x", "Mute".to_string(), ui.muted)
                } else {
                    ("volume-2", format!("{}%", s.volume), ui.text)
                };
                metric(
                    c,
                    &mut right,
                    &mut hits,
                    icon,
                    &text,
                    color,
                    Some(Action::VolumeMute),
                );
                if s.media_playing && !s.media_title.is_empty() {
                    let display_title = if s.media_title.chars().count() > 20 {
                        format!("{}…", s.media_title.chars().take(19).collect::<String>())
                    } else {
                        s.media_title.clone()
                    };
                    metric(
                        c,
                        &mut right,
                        &mut hits,
                        "play",
                        &display_title,
                        ui.muted,
                        Some(Action::MediaPlayPause),
                    );
                }
            }
            "caffeine" if settings.show_caffeine => {
                let (icon, text, color) = if s.caffeine {
                    ("coffee", "ON", ui.warning)
                } else {
                    ("coffee", "", ui.muted)
                };
                if text.is_empty() {
                    let r = Rect::new(right - c.p(24.), c.p(8.), c.p(24.), h - c.p(16.));
                    c.icon(
                        icon,
                        Rect::new(r.x + c.p(4.), (h - c.p(16.)) / 2, c.p(16.), c.p(16.)),
                        color,
                    );
                    hit(&mut hits, r, Action::CaffeineToggle);
                    right = r.x - c.p(12.);
                } else {
                    metric(
                        c,
                        &mut right,
                        &mut hits,
                        icon,
                        text,
                        color,
                        Some(Action::CaffeineToggle),
                    );
                }
            }
            "tiling" if settings.show_tiling => {
                let r = Rect::new(right - c.p(30.), c.p(8.), c.p(30.), h - c.p(16.));
                c.icon(
                    if vertical {
                        "arrow-up-down"
                    } else {
                        "arrow-left-right"
                    },
                    Rect::new(r.x + c.p(7.), r.y + c.p(4.), c.p(16.), c.p(16.)),
                    ui.text,
                );
                hit(
                    &mut hits,
                    r,
                    Action::Command("toggle-tiling-direction".into()),
                );
                right = r.x - c.p(12.);
            }
            "clock" if settings.show_clock => {
                let dw = c.measure(clock, 12.5, true);
                let cr = Rect::new(right - dw - c.p(8.), 0, dw + c.p(8.), h);
                c.text(clock, cr, ui.text, 12.5, true, 1);
                hit(
                    &mut hits,
                    cr,
                    Action::Settings(crate::settings_ui::Edit::CycleClockFormat),
                );
                right = cr.x - c.p(12.);
            }
            "scale" if settings.show_scale => {
                let sr = Rect::new(right - c.p(28.), (h - c.p(28.)) / 2, c.p(28.), c.p(28.));
                if is_transparent {
                    c.frame(sr, 0x00888888);
                    c.icon(
                        "scaling",
                        Rect::new(sr.x + c.p(6.), sr.y + c.p(6.), c.p(16.), c.p(16.)),
                        0x00FFFFFF,
                    );
                } else {
                    c.fill(sr, ui.surface);
                    c.icon(
                        "scaling",
                        Rect::new(sr.x + c.p(6.), sr.y + c.p(6.), c.p(16.), c.p(16.)),
                        ui.muted,
                    );
                }
                hit(&mut hits, sr, Action::Scale);
                right = sr.x - c.p(12.);
            }
            "brightness" if settings.show_brightness => {
                let br = Rect::new(right - c.p(150.), 0, c.p(150.), h);
                paint_brightness(c, br, brightness);
                if brightness.is_some() {
                    hit(&mut hits, br, Action::Brightness);
                }
                right = br.x - c.p(12.);
            }
            _ => {}
        }
    }

    if paused {
        let r = Rect::new(right - c.p(78.), c.p(8.), c.p(78.), h - c.p(16.));
        button(
            c,
            &mut hits,
            "일시 중지",
            r,
            true,
            false,
            true,
            Action::Command("wm-toggle-pause".into()),
        );
        right -= c.p(90.);
    }
    if resize {
        button(
            c,
            &mut hits,
            "크기 조절 중",
            Rect::new(right - c.p(100.), c.p(8.), c.p(100.), h - c.p(16.)),
            true,
            false,
            true,
            Action::Command("wm-disable-binding-mode --name resize".into()),
        );
    }
    if !is_transparent {
        c.fill(Rect::new(0, h - 1, c.width, 1), ui.border);
    }
    c.ui = saved_ui;
    hits
}
pub fn clock_formatted(idx: usize) -> String {
    let now = chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(9 * 3600).unwrap());
    let day =
        ["일", "월", "화", "수", "목", "금", "토"][now.weekday().num_days_from_sunday() as usize];
    match idx {
        1 => format!("{:02}:{:02}", now.hour(), now.minute()),
        2 => format!(
            "{}년 {}월 {}일 ({})",
            now.year(),
            now.month(),
            now.day(),
            day
        ),
        3 => format!(
            "{}월 {}일 {:02}:{:02}",
            now.month(),
            now.day(),
            now.hour(),
            now.minute()
        ),
        4 => format!(
            "{}-{:02}-{:02} {:02}:{:02}",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute()
        ),
        _ => format!(
            "{}년 {}월 {}일 ({}) {:02}:{:02}",
            now.year(),
            now.month(),
            now.day(),
            day,
            now.hour(),
            now.minute()
        ),
    }
}

pub fn clock() -> String {
    clock_formatted(0)
}
pub fn button(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    text: &str,
    r: Rect,
    selected: bool,
    hover: bool,
    enabled: bool,
    action: Action,
) {
    let ui = c.ui;
    c.fill(
        r,
        if selected {
            ui.selected
        } else if hover && enabled {
            ui.hover
        } else {
            ui.surface
        },
    );
    c.frame(
        r,
        if selected || (hover && enabled) {
            ui.accent
        } else {
            ui.border
        },
    );
    c.text(
        text,
        r,
        if enabled {
            ui.text
        } else {
            crate::graphics::blend(ui.bg, ui.text, 0.38)
        },
        12.,
        false,
        1,
    );
    if enabled {
        hit(hits, r, action)
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct DropdownOption {
    pub label: String,
    pub selected: bool,
    pub swatch: Option<u32>,
}

impl DropdownOption {
    pub fn new(label: impl Into<String>, selected: bool) -> Self {
        Self {
            label: label.into(),
            selected,
            swatch: None,
        }
    }
    #[allow(dead_code)]
    pub fn swatch(mut self, color: u32) -> Self {
        self.swatch = Some(color);
        self
    }
}

/// Renders the shared dropdown menu and returns its item hit rects plus the
/// full-window dismiss rect.
///
/// Item rects come first so clicks reach the menu even though the dismiss
/// target follows them. Screens append their existing hits after the dismiss
/// target, which keeps it above the underlying controls.
pub fn dropdown(
    c: &mut Canvas,
    anchor: Rect,
    options: &[DropdownOption],
    hover: Option<(i32, i32)>,
) -> (Vec<(Rect, usize)>, Rect) {
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let pad = s(4.);
    let item_h = s(26.);
    let mut menu_w = anchor.w.max(s(200.));
    for option in options {
        let mut width = s(32.);
        if option.swatch.is_some() {
            width += s(18.);
        }
        width += c.measure(&option.label, 11.5, false);
        menu_w = menu_w.max(width + s(20.));
    }
    let menu_w = menu_w.min(c.width - 2 * pad);
    let menu_h = item_h * options.len() as i32 + 2 * pad;
    let menu_x = anchor.x.clamp(pad, (c.width - pad - menu_w).max(pad));
    let menu_y = if anchor.y + anchor.h + s(2.) + menu_h <= c.height - pad {
        anchor.y + anchor.h + s(2.)
    } else {
        (anchor.y - menu_h - s(2.)).max(pad)
    };
    let menu = Rect::new(menu_x, menu_y, menu_w, menu_h);
    c.fill(menu, c.ui.surface);
    c.frame(menu, c.ui.border);

    let mut hits = Vec::with_capacity(options.len());
    for (index, option) in options.iter().enumerate() {
        let r = Rect::new(
            menu_x + pad,
            menu_y + pad + index as i32 * item_h,
            menu_w - 2 * pad,
            item_h,
        );
        if option.selected {
            c.fill(r, c.ui.selected);
        } else if hover.is_some_and(|(x, y)| r.contains(x, y)) {
            c.fill(r, c.ui.hover);
        }
        if option.selected {
            c.text(
                "✓",
                Rect::new(r.x + s(6.), r.y, s(14.), r.h),
                c.ui.accent,
                11.5,
                true,
                0,
            );
        }
        let mut text_x = r.x + s(24.);
        if let Some(color) = option.swatch {
            let chip = s(10.);
            let chip_rect = Rect::new(text_x, r.y + (r.h - chip) / 2, chip, chip);
            c.fill(chip_rect, color);
            c.frame(chip_rect, crate::graphics::blend(color, c.ui.border, 0.5));
            text_x += chip + s(8.);
        }
        c.text(
            &option.label,
            Rect::new(text_x, r.y, r.x + r.w - text_x - s(8.), r.h),
            if option.selected {
                c.ui.accent
            } else {
                c.ui.text
            },
            11.5,
            option.selected,
            0,
        );
        hits.push((r, index));
    }
    (hits, Rect::new(0, 0, c.width, c.height))
}

pub fn paint_brightness(c: &mut Canvas, r: Rect, value: Option<u8>) {
    let ui = c.ui;
    let cy = r.y + r.h / 2;
    let start = r.x + c.p(25.);
    let width = r.w - c.p(66.);
    c.icon(
        "sun",
        Rect::new(r.x, r.y + (r.h - c.p(16.)) / 2, c.p(16.), c.p(16.)),
        if value.is_some() { ui.muted } else { 0x696969 },
    );
    c.fill(Rect::new(start, cy - c.p(1.), width, c.p(2.)), ui.border);
    if let Some(v) = value {
        c.fill(
            Rect::new(start, cy - c.p(1.), width * i32::from(v) / 100, c.p(2.)),
            ui.accent,
        );
    }
    for i in 0..=10 {
        c.fill(
            Rect::new(
                start + width * i / 10 - c.p(1.),
                cy + c.p(6.),
                c.p(2.),
                c.p(2.),
            ),
            ui.border,
        )
    }
    if let Some(v) = value {
        let x = start + width * i32::from(v) / 100;
        c.fill(
            Rect::new(x - c.p(3.), cy - c.p(5.), c.p(6.), c.p(10.)),
            ui.text,
        );
    }
    c.text(
        &value.map(|v| format!("{v}%")).unwrap_or("—".into()),
        Rect::new(r.x + r.w - c.p(35.), r.y, c.p(35.), r.h),
        ui.muted,
        12.,
        false,
        2,
    );
}
pub const SCALES: [i32; 12] = [100, 125, 150, 175, 200, 225, 250, 300, 350, 400, 450, 500];
pub fn scale(c: &mut Canvas, current: usize, recommended: usize, max: usize, error: &str) {
    let ui = c.ui;
    let r = |c: &Canvas, x: f64, y: f64, w: f64, h: f64| Rect::new(c.p(x), c.p(y), c.p(w), c.p(h));
    c.text(
        "디스플레이 배율",
        r(c, 16., 12., 160., 24.),
        ui.text,
        13.,
        true,
        0,
    );
    c.text(
        &format!("{}%", SCALES[current]),
        Rect::new(c.width - c.p(80.), c.p(12.), c.p(64.), c.p(24.)),
        ui.accent,
        13.,
        true,
        2,
    );
    let left = c.p(16.);
    let width = c.width - c.p(32.);
    let cy = c.p(60.) + c.p(4.) / 2;
    let x = left + width * current as i32 / max.max(1) as i32;
    c.fill(Rect::new(left, cy - c.p(2.), width, c.p(4.)), ui.border);
    c.fill(Rect::new(left, cy - c.p(2.), x - left, c.p(4.)), ui.accent);
    for i in 0..=max {
        let d = left + width * i as i32 / max.max(1) as i32;
        c.ellipse(
            Rect::new(d - c.p(2.), cy - c.p(2.), c.p(4.), c.p(4.)),
            if i <= current { ui.accent } else { ui.border },
        );
    }
    c.ellipse(
        Rect::new(x - c.p(5.), cy - c.p(5.), c.p(10.), c.p(10.)),
        ui.text,
    );
    let label = Rect::new(c.p(16.), c.p(82.), c.width - c.p(32.), c.p(18.));
    if error.is_empty() {
        c.text("100%", label, ui.muted, 11., false, 0);
        c.text(
            &format!("권장 {}%", SCALES[recommended]),
            label,
            ui.muted,
            11.,
            false,
            1,
        );
        c.text(&format!("{}%", SCALES[max]), label, ui.muted, 11., false, 2);
    } else {
        c.text(error, label, ui.error, 11., false, 0);
    }
    c.frame(Rect::new(0, 0, c.width, c.height), ui.border);
}
pub fn battery(c: &mut Canvas, s: &Sensors) {
    let ui = c.ui;
    let compact = if s.discharging && !s.charging {
        22.
    } else {
        0.
    };
    let col = if s.charging {
        ui.success
    } else if s.discharging {
        ui.warning
    } else {
        ui.muted
    };
    let value = if s.known {
        format!(
            "{}{:.1} W",
            if s.charging {
                "+"
            } else if s.discharging {
                "−"
            } else {
                ""
            },
            s.watts
        )
    } else {
        "— W".into()
    };
    c.icon(
        "zap",
        Rect::new(c.p(16.), c.p(21.), c.p(18.), c.p(18.)),
        col,
    );
    c.text(
        &value,
        Rect::new(c.p(44.), c.p(12.), c.width - c.p(60.), c.p(36.)),
        col,
        25.,
        false,
        0,
    );
    let label = if s.charging {
        "충전 중"
    } else if s.discharging {
        ""
    } else {
        "대기 중"
    };
    c.text(
        label,
        Rect::new(c.p(46.), c.p(49.), c.width - c.p(60.), c.p(18.)),
        ui.muted,
        12.,
        false,
        0,
    );
    c.fill(
        Rect::new(c.p(16.), c.p(73. - compact), c.width - c.p(32.), 1),
        ui.border,
    );
    let remaining = if s.known && s.watts > 0. && (s.charging || s.discharging) {
        let hours = if s.charging {
            (s.full - s.remaining).max(0.) / s.watts
        } else {
            s.remaining / s.watts
        };
        if (0.0..1000.0).contains(&hours) {
            let mins = (hours * 60.).round_ties_even() as i32;
            format!("약 {}시간 {}분", mins / 60, mins % 60)
        } else {
            "—".into()
        }
    } else {
        "—".into()
    };
    for (label, text, y) in [
        (
            "배터리",
            format!("{:.1} / {:.1} Wh", s.remaining, s.full),
            82. - compact,
        ),
        (
            if s.charging {
                "완충까지"
            } else {
                "남은 시간"
            },
            remaining,
            105. - compact,
        ),
    ] {
        c.text(
            label,
            Rect::new(c.p(16.), c.p(y), c.p(72.), c.p(18.)),
            ui.muted,
            12.,
            false,
            0,
        );
        c.text(
            &text,
            Rect::new(c.p(66.), c.p(y), c.width - c.p(82.), c.p(18.)),
            ui.text,
            12.,
            false,
            2,
        );
    }
    c.frame(Rect::new(0, 0, c.width, c.height), ui.border);
}
pub fn power_button(c: &mut Canvas, r: Rect, index: usize, selected: bool, hover: bool) {
    let ui = c.ui;
    c.fill(
        r,
        if selected {
            ui.selected
        } else if hover {
            ui.hover
        } else {
            ui.bg
        },
    );
    let col = [ui.warning, ui.accent, ui.success][index];
    if selected {
        c.fill(Rect::new(r.x, r.y + c.p(10.), c.p(2.), r.h - c.p(20.)), col);
    }
    c.icon(
        ["zap", "arrow-left-right", "battery-full"][index],
        Rect::new(
            r.x + c.p(14.),
            r.y + (r.h - c.p(17.)) / 2,
            c.p(17.),
            c.p(17.),
        ),
        col,
    );
    c.text(
        ["고성능", "균형", "절전"][index],
        Rect::new(r.x + c.p(44.), r.y, r.w - c.p(76.), r.h),
        ui.text,
        14.,
        false,
        0,
    );
    if selected || hover {
        c.text(
            if selected { "✓" } else { ">" },
            Rect::new(r.x + r.w - c.p(32.), r.y, c.p(24.), r.h),
            col,
            if selected { 18. } else { 14. },
            false,
            1,
        );
    }
}
pub fn power(c: &mut Canvas, p: &crate::power::Profiles, hover: Option<(i32, i32)>) -> Vec<Hit> {
    let mut hits = vec![];
    for (i, id) in p.ids.iter().enumerate() {
        let r = Rect::new(c.p(8.), c.p(8. + i as f64 * 40.), c.p(224.), c.p(40.));
        power_button(
            c,
            r,
            i,
            id == &p.active,
            !id.is_empty() && hover.is_some_and(|(x, y)| r.contains(x, y)),
        );
        if id.is_empty() {
            c.text(
                "—",
                Rect::new(r.x + r.w - c.p(32.), r.y, c.p(24.), r.h),
                c.ui.border,
                14.,
                false,
                1,
            );
        } else {
            hit(&mut hits, r, Action::PowerSelect(id.clone()));
        }
    }
    if !p.error.is_empty() {
        c.text(
            &p.error,
            Rect::new(c.p(8.), c.height - c.p(23.), c.width - c.p(16.), c.p(20.)),
            c.ui.error,
            10.,
            false,
            0,
        );
    }
    c.frame(Rect::new(0, 0, c.width, c.height), c.ui.border);
    hits
}
