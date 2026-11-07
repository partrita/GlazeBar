use crate::{
    graphics::{Canvas, Rect, blend},
    views::{self, Action, Hit},
    weather::CITIES,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

pub const APP_LABELS: [&str; 16] = [
    "GlazeBar",
    "Terminal",
    "Codex CLI",
    "VS Code",
    "Rio Terminal",
    "Zed Editor",
    "Antigravity",
    "Claude",
    "OpenCode",
    "Firefox",
    "Chrome",
    "Edge",
    "WezTerm",
    "Alacritty",
    "Hyper",
    "PuTTY",
];

pub const TERMINALS: [(&str, &str); 6] = [
    ("Windows Terminal", "shell-exec wt"),
    ("Rio Terminal", "shell-exec rio"),
    ("PowerShell", "shell-exec pwsh"),
    ("Alacritty", "shell-exec alacritty"),
    ("WezTerm", "shell-exec wezterm-gui"),
    ("Hyper", "shell-exec hyper"),
];

pub const BROWSERS: [(&str, &str); 4] = [
    (
        "기본 브라우저",
        "shell-exec cmd /c start \"\" \"https://www.google.com\"",
    ),
    (
        "Chrome",
        "shell-exec cmd /c start chrome \"https://www.google.com\"",
    ),
    (
        "Edge",
        "shell-exec cmd /c start msedge \"https://www.google.com\"",
    ),
    (
        "Firefox",
        "shell-exec cmd /c start firefox \"https://www.google.com\"",
    ),
];

pub const GAPS: [(&str, &str); 4] = [
    ("없음 (0px)", "0px"),
    ("좁게 (8px)", "8px"),
    ("기본 (16px)", "16px"),
    ("넓게 (24px)", "24px"),
];

pub const CLOCK_FORMATS: [(&str, &str); 5] = [
    ("전체", "년/월/일 요일 시:분"),
    ("시간만", "시:분"),
    ("날짜만", "년/월/일 요일"),
    ("간략형", "월/일 시:분"),
    ("ISO", "YYYY-MM-DD HH:MM"),
];

pub const OPACITY_STEPS: [(&str, u8); 6] = [
    ("100% (불투명)", 100),
    ("80%", 80),
    ("60%", 60),
    ("40%", 40),
    ("20%", 20),
    ("0% (투명)", 0),
];

pub const TOPBAR_WIDGET_IDS: [&str; 11] = [
    "weather",
    "battery",
    "cpu",
    "memory",
    "wifi",
    "volume",
    "caffeine",
    "tiling",
    "clock",
    "scale",
    "brightness",
];

pub const TOPBAR_WIDGET_NAMES: [&str; 11] = [
    "날씨 정보",
    "배터리 / 전력",
    "CPU 사용률",
    "메모리 사용률",
    "Wi-Fi 네트워크",
    "볼륨 & 미디어",
    "카페인 (절전 방지)",
    "창 분할 아이콘",
    "시계 (날짜/시간)",
    "디스플레이 배율",
    "화면 밝기 조절",
];

fn default_widget_order() -> Vec<String> {
    TOPBAR_WIDGET_IDS.iter().map(|&s| s.to_string()).collect()
}

fn default_true() -> bool {
    true
}
fn default_targets() -> [bool; 16] {
    [true; 16]
}
fn default_gap_idx() -> usize {
    2
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SettingsDropdown {
    WidgetVisibility(usize),
    City,
    ClockFormat,
    Opacity,
    Terminal,
    Browser,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BarSettings {
    #[serde(default = "default_true")]
    pub show_weather: bool,
    #[serde(default = "default_true")]
    pub show_cpu: bool,
    #[serde(default = "default_true")]
    pub show_memory: bool,
    #[serde(default = "default_true")]
    pub show_battery: bool,
    #[serde(default = "default_true")]
    pub show_wifi: bool,
    #[serde(default = "default_true")]
    pub show_volume: bool,
    #[serde(default = "default_true")]
    pub show_caffeine: bool,
    #[serde(default = "default_true")]
    pub show_brightness: bool,
    #[serde(default = "default_true")]
    pub show_scale: bool,
    #[serde(default = "default_true")]
    pub show_tiling: bool,
    #[serde(default = "default_true")]
    pub show_clock: bool,
    #[serde(default)]
    pub show_quick_buttons: bool,
    #[serde(default = "default_widget_order")]
    pub widget_order: Vec<String>,
    #[serde(default)]
    pub weather_city: usize,
    #[serde(default)]
    pub clock_format_idx: usize,
    #[serde(default)]
    pub opacity_idx: usize,
    #[serde(default = "default_true")]
    pub auto_split: bool,
    #[serde(default)]
    pub terminal_idx: usize,
    #[serde(default)]
    pub browser_idx: usize,
    #[serde(default = "default_gap_idx")]
    pub gap_idx: usize,
    #[serde(default = "default_targets")]
    pub targets: [bool; 16],
    #[serde(skip)]
    pub status: String,
    #[serde(skip)]
    pub open_dropdown: Option<SettingsDropdown>,
}

impl Default for BarSettings {
    fn default() -> Self {
        Self {
            show_weather: true,
            show_cpu: true,
            show_memory: true,
            show_battery: true,
            show_wifi: true,
            show_volume: true,
            show_caffeine: true,
            show_brightness: true,
            show_scale: true,
            show_tiling: true,
            show_clock: true,
            show_quick_buttons: false,
            widget_order: default_widget_order(),
            weather_city: 0,
            clock_format_idx: 0,
            opacity_idx: 0,
            auto_split: true,
            terminal_idx: 0,
            browser_idx: 0,
            gap_idx: 2,
            targets: [true; 16],
            status: String::new(),
            open_dropdown: None,
        }
    }
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum Edit {
    ToggleWeather,
    ToggleCpu,
    ToggleMemory,
    ToggleBattery,
    ToggleWifi,
    ToggleVolume,
    ToggleCaffeine,
    ToggleBrightness,
    ToggleScale,
    ToggleQuickButtons,
    SetWidgetVisible(String, bool),
    MoveWidget(usize, isize),
    ResetWidgetOrder,
    ToggleDropdown(SettingsDropdown),
    CloseDropdown,
    SetCity(usize),
    SetClockFormat(usize),
    SetOpacity(usize),
    SetTerminal(usize),
    SetBrowser(usize),
    SetAutoSplit(bool),
    CycleCity,
    CycleClockFormat,
    CycleOpacity,
    ToggleAutoSplit,
    CycleTerminal,
    CycleBrowser,
    ToggleTarget(usize),
    SelectAllTargets(bool),
    OpenConfig,
    ReloadGlazeWM,
    OpenSheet,
    Save,
}

impl BarSettings {
    pub fn is_widget_visible(&self, id: &str) -> bool {
        match id {
            "weather" => self.show_weather,
            "battery" => self.show_battery,
            "cpu" => self.show_cpu,
            "memory" => self.show_memory,
            "wifi" => self.show_wifi,
            "volume" => self.show_volume,
            "caffeine" => self.show_caffeine,
            "brightness" => self.show_brightness,
            "scale" => self.show_scale,
            "tiling" => self.show_tiling,
            "clock" => self.show_clock,
            _ => true,
        }
    }

    pub fn set_widget_visible(&mut self, id: &str, visible: bool) {
        match id {
            "weather" => self.show_weather = visible,
            "battery" => self.show_battery = visible,
            "cpu" => self.show_cpu = visible,
            "memory" => self.show_memory = visible,
            "wifi" => self.show_wifi = visible,
            "volume" => self.show_volume = visible,
            "caffeine" => self.show_caffeine = visible,
            "brightness" => self.show_brightness = visible,
            "scale" => self.show_scale = visible,
            "tiling" => self.show_tiling = visible,
            "clock" => self.show_clock = visible,
            _ => {}
        }
    }

    pub fn normalized_widget_order(&self) -> Vec<String> {
        let mut order = self.widget_order.clone();
        for &id in &TOPBAR_WIDGET_IDS {
            if !order.iter().any(|item| item == id) {
                order.push(id.to_string());
            }
        }
        order.retain(|id| TOPBAR_WIDGET_IDS.contains(&id.as_str()));
        order
    }

    pub fn load(root: &Path) -> Self {
        let path = root.join("theme-state/glazebar-settings.json");
        let mut s: Self = std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        s.widget_order = s.normalized_widget_order();
        s.weather_city = s.weather_city.min(CITIES.len().saturating_sub(1));
        s.clock_format_idx = s
            .clock_format_idx
            .min(CLOCK_FORMATS.len().saturating_sub(1));
        s.opacity_idx = s.opacity_idx.min(OPACITY_STEPS.len().saturating_sub(1));
        s.terminal_idx = s.terminal_idx.min(TERMINALS.len().saturating_sub(1));
        s.browser_idx = s.browser_idx.min(BROWSERS.len().saturating_sub(1));
        s.gap_idx = s.gap_idx.min(GAPS.len().saturating_sub(1));
        s.auto_split = !root.join("autotile.disabled").exists();
        s.open_dropdown = None;
        s
    }

    pub fn save(&self, root: &Path) -> anyhow::Result<()> {
        crate::theme_transaction::atomic_write(
            &root.join("theme-state/glazebar-settings.json"),
            &serde_json::to_vec_pretty(self)?,
        )?;
        let autotile_flag = root.join("autotile.disabled");
        if self.auto_split {
            if autotile_flag.exists() {
                let _ = std::fs::remove_file(autotile_flag);
            }
        } else {
            let _ = std::fs::write(autotile_flag, "disabled");
        }
        let config_path = root.join("config.yaml");
        if let Ok(source) = std::fs::read_to_string(&config_path) {
            let mut updated = source.clone();
            if let Some((_, cmd)) = TERMINALS.get(self.terminal_idx) {
                updated =
                    crate::theme_text::glazewm_set_binding_command(&updated, "alt+enter", cmd);
            }
            if let Some((_, cmd)) = BROWSERS.get(self.browser_idx) {
                updated = crate::theme_text::glazewm_set_binding_command(
                    &updated,
                    "alt+shift+enter",
                    cmd,
                );
            }
            if updated != source {
                let _ = crate::theme_transaction::atomic_write(&config_path, updated.as_bytes());
            }
        }
        Ok(())
    }

    pub fn apply_edit(&mut self, edit: &Edit) {
        match edit {
            Edit::ToggleWeather => self.show_weather = !self.show_weather,
            Edit::ToggleCpu => self.show_cpu = !self.show_cpu,
            Edit::ToggleMemory => self.show_memory = !self.show_memory,
            Edit::ToggleBattery => self.show_battery = !self.show_battery,
            Edit::ToggleWifi => self.show_wifi = !self.show_wifi,
            Edit::ToggleVolume => self.show_volume = !self.show_volume,
            Edit::ToggleCaffeine => self.show_caffeine = !self.show_caffeine,
            Edit::ToggleBrightness => self.show_brightness = !self.show_brightness,
            Edit::ToggleScale => self.show_scale = !self.show_scale,
            Edit::ToggleQuickButtons => self.show_quick_buttons = !self.show_quick_buttons,
            Edit::SetWidgetVisible(id, visible) => {
                self.set_widget_visible(id, *visible);
                self.open_dropdown = None;
                self.status = format!(
                    "✓ 위젯 표시 설정을 변경했습니다 ({})",
                    if *visible { "표시" } else { "숨김" }
                );
                return;
            }
            Edit::MoveWidget(idx, delta) => {
                let mut order = self.normalized_widget_order();
                let new_idx = (*idx as isize + delta).clamp(0, order.len() as isize - 1) as usize;
                if new_idx != *idx && *idx < order.len() {
                    let item = order.remove(*idx);
                    order.insert(new_idx, item);
                    self.widget_order = order;
                    self.open_dropdown = None;
                    self.status = "✓ 상단 바 위젯 순서를 변경했습니다".into();
                    return;
                }
            }
            Edit::ResetWidgetOrder => {
                self.widget_order = default_widget_order();
                self.open_dropdown = None;
                self.status = "✓ 위젯 순서를 기본값으로 복원했습니다".into();
                return;
            }
            Edit::ToggleDropdown(d) => {
                self.open_dropdown = if self.open_dropdown == Some(*d) {
                    None
                } else {
                    Some(*d)
                };
            }
            Edit::CloseDropdown => {
                self.open_dropdown = None;
            }
            Edit::SetCity(idx) => {
                self.weather_city = (*idx).min(CITIES.len().saturating_sub(1));
                self.open_dropdown = None;
                self.status = format!("✓ 날씨 측정 지역: {}", CITIES[self.weather_city].name);
                return;
            }
            Edit::SetClockFormat(idx) => {
                self.clock_format_idx = (*idx).min(CLOCK_FORMATS.len().saturating_sub(1));
                self.open_dropdown = None;
                self.status = format!(
                    "✓ 시계 표시 형식: {} ({})",
                    CLOCK_FORMATS[self.clock_format_idx].0, CLOCK_FORMATS[self.clock_format_idx].1
                );
                return;
            }
            Edit::SetOpacity(idx) => {
                self.opacity_idx = (*idx).min(OPACITY_STEPS.len().saturating_sub(1));
                self.open_dropdown = None;
                self.status = format!("✓ 상단 바 투명도: {}", OPACITY_STEPS[self.opacity_idx].0);
                return;
            }
            Edit::SetTerminal(idx) => {
                self.terminal_idx = (*idx).min(TERMINALS.len().saturating_sub(1));
                self.open_dropdown = None;
                self.status = format!(
                    "✓ 기본 터미널 (Alt+Enter): {}",
                    TERMINALS[self.terminal_idx].0
                );
                return;
            }
            Edit::SetBrowser(idx) => {
                self.browser_idx = (*idx).min(BROWSERS.len().saturating_sub(1));
                self.open_dropdown = None;
                self.status = format!(
                    "✓ 기본 브라우저 (Alt+Shift+Enter): {}",
                    BROWSERS[self.browser_idx].0
                );
                return;
            }
            Edit::SetAutoSplit(on) => {
                self.auto_split = *on;
                self.status = format!("✓ 창 비율 자동 분할: {}", if *on { "켜짐" } else { "꺼짐" });
                return;
            }
            Edit::CycleCity => self.weather_city = (self.weather_city + 1) % CITIES.len(),
            Edit::CycleClockFormat => {
                self.clock_format_idx = (self.clock_format_idx + 1) % CLOCK_FORMATS.len()
            }
            Edit::CycleOpacity => self.opacity_idx = (self.opacity_idx + 1) % OPACITY_STEPS.len(),
            Edit::ToggleAutoSplit => self.auto_split = !self.auto_split,
            Edit::CycleTerminal => self.terminal_idx = (self.terminal_idx + 1) % TERMINALS.len(),
            Edit::CycleBrowser => self.browser_idx = (self.browser_idx + 1) % BROWSERS.len(),
            Edit::ToggleTarget(i) => {
                if let Some(t) = self.targets.get_mut(*i) {
                    *t = !*t;
                }
            }
            Edit::SelectAllTargets(val) => {
                self.targets = [*val; 16];
            }
            _ => {}
        }
        self.status.clear();
    }
}

fn draw_widget_card(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    r: Rect,
    rank: usize,
    name: &str,
    id: &str,
    visible: bool,
    hover: Option<(i32, i32)>,
) {
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let arrow_w = s(24.);
    let main_w = r.w - arrow_w * 2 - s(4.);
    let main_r = Rect::new(r.x, r.y, main_w, r.h);
    let left_r = Rect::new(r.x + main_w + s(2.), r.y, arrow_w, r.h);
    let right_r = Rect::new(r.x + main_w + s(4.) + arrow_w, r.y, arrow_w, r.h);

    let main_over = hover.is_some_and(|(x, y)| main_r.contains(x, y));
    let bg = if visible {
        if main_over {
            blend(c.ui.selected, c.ui.accent, 0.22)
        } else {
            blend(c.ui.surface, c.ui.accent, 0.14)
        }
    } else if main_over {
        c.ui.hover
    } else {
        c.ui.bg
    };
    c.fill(main_r, bg);
    c.frame(
        main_r,
        if visible {
            blend(c.ui.border, c.ui.accent, 0.55)
        } else {
            c.ui.border
        },
    );
    // Left indicator bar
    c.fill(
        Rect::new(main_r.x, main_r.y, s(4.), main_r.h),
        if visible { c.ui.accent } else { c.ui.border },
    );

    // Widget rank & name
    let label = format!("{}. {}", rank + 1, name);
    c.text(
        &label,
        Rect::new(main_r.x + s(12.), main_r.y, main_r.w - s(54.), main_r.h),
        if visible { c.ui.text } else { c.ui.muted },
        11.5,
        visible,
        0,
    );

    // Status pill ("ON" / "OFF")
    let badge_w = s(36.);
    let badge_h = s(18.);
    let badge_r = Rect::new(
        main_r.x + main_r.w - badge_w - s(8.),
        main_r.y + (main_r.h - badge_h) / 2,
        badge_w,
        badge_h,
    );
    c.fill(badge_r, if visible { c.ui.accent } else { c.ui.surface });
    c.text(
        if visible { "표시" } else { "숨김" },
        badge_r,
        if visible { 0x000000 } else { c.ui.muted },
        10.,
        true,
        1,
    );

    hits.push(Hit {
        rect: main_r,
        action: Action::Settings(Edit::SetWidgetVisible(id.to_string(), !visible)),
    });

    // Reorder buttons (◀ ▶)
    views::button(
        c,
        hits,
        "◀",
        left_r,
        false,
        hover.is_some_and(|(x, y)| left_r.contains(x, y)),
        rank > 0,
        Action::Settings(Edit::MoveWidget(rank, -1)),
    );
    views::button(
        c,
        hits,
        "▶",
        right_r,
        false,
        hover.is_some_and(|(x, y)| right_r.contains(x, y)),
        rank + 1 < TOPBAR_WIDGET_IDS.len(),
        Action::Settings(Edit::MoveWidget(rank, 1)),
    );
}

fn draw_segmented_row(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    label: &str,
    y: i32,
    w: i32,
    options: &[(&str, bool, Edit)],
    hover: Option<(i32, i32)>,
) {
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let label_w = s(170.);
    let row_h = s(30.);
    c.text(
        label,
        Rect::new(s(38.), y, label_w, row_h),
        c.ui.text,
        12.,
        true,
        0,
    );

    let avail_w = w - s(38.) - label_w - s(38.);
    let count = options.len().max(1) as i32;
    let gap = s(6.);
    let item_w = (avail_w - gap * (count - 1)) / count;

    for (i, (text, selected, edit)) in options.iter().enumerate() {
        let x = s(38.) + label_w + i as i32 * (item_w + gap);
        let r = Rect::new(x, y, item_w, row_h);
        let over = hover.is_some_and(|(hx, hy)| r.contains(hx, hy));
        let bg = if *selected {
            c.ui.accent
        } else if over {
            c.ui.hover
        } else {
            c.ui.bg
        };
        c.fill(r, bg);
        c.frame(r, if *selected { c.ui.accent } else { c.ui.border });
        c.text(
            text,
            r,
            if *selected { 0x000000 } else { c.ui.text },
            11.5,
            *selected,
            1,
        );
        hits.push(Hit {
            rect: r,
            action: Action::Settings(edit.clone()),
        });
    }
}

/// `chrome` is false when the screen is embedded in the settings hub, which
/// draws its own close button in the shared tab strip.
pub fn paint(c: &mut Canvas, v: &BarSettings, hover: Option<(i32, i32)>, chrome: bool) -> Vec<Hit> {
    let mut hits = vec![];
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let w = c.width;
    let h = c.height;

    if chrome {
        let close_rect = Rect::new(w - s(96.), s(12.), s(72.), s(28.));
        views::button(
            c,
            &mut hits,
            "닫기",
            close_rect,
            false,
            hover.is_some_and(|(x, y)| close_rect.contains(x, y)),
            true,
            Action::Close,
        );
    }

    // -------------------------------------------------------------------------
    // Card 1: 상단 바 위젯 표시 및 배치 순서 (원클릭 토글 + ◀ ▶ 순서 변경)
    // -------------------------------------------------------------------------
    let sec1 = Rect::new(s(24.), s(14.), w - s(48.), s(166.));
    c.round_rect(sec1, c.ui.surface, s(8.));
    c.text(
        "상단 바 위젯 표시 및 배치 순서",
        Rect::new(s(38.), s(22.), s(240.), s(24.)),
        c.ui.accent,
        12.5,
        true,
        0,
    );
    c.text(
        "카드를 클릭해 즉시 켜거나 끄고, 우측 ◀ ▶ 버튼으로 상단 바 표시 순서를 조정하세요 (오른쪽 끝부터 1번)",
        Rect::new(s(270.), s(22.), w - s(440.), s(24.)),
        c.ui.muted,
        11.,
        false,
        0,
    );

    let reset_order_r = Rect::new(w - s(164.), s(21.), s(126.), s(26.));
    views::button(
        c,
        &mut hits,
        "순서 초기화 ↺",
        reset_order_r,
        false,
        hover.is_some_and(|(x, y)| reset_order_r.contains(x, y)),
        true,
        Action::Settings(Edit::ResetWidgetOrder),
    );

    let order = v.normalized_widget_order();
    let grid_w = w - s(76.);
    let col_gap = s(10.);
    let col_w = (grid_w - col_gap * 2) / 3;
    let row_h = s(34.);
    let row_gap = s(8.);

    for (i, widget_id) in order.iter().enumerate() {
        let row = i / 3;
        let col = i % 3;
        let item_r = Rect::new(
            s(38.) + col as i32 * (col_w + col_gap),
            s(54.) + row as i32 * (row_h + row_gap),
            col_w,
            row_h,
        );
        let is_visible = v.is_widget_visible(widget_id);
        let name_idx = TOPBAR_WIDGET_IDS
            .iter()
            .position(|&id| id == widget_id)
            .unwrap_or(0);
        let display_name = TOPBAR_WIDGET_NAMES[name_idx];
        draw_widget_card(
            c,
            &mut hits,
            item_r,
            i,
            display_name,
            widget_id,
            is_visible,
            hover,
        );
    }

    // -------------------------------------------------------------------------
    // Card 2: 상단 바 외형 · 시계 형식 · 날씨 지역 (드롭다운 없는 원클릭 세그먼트 바)
    // -------------------------------------------------------------------------
    let sec2 = Rect::new(s(24.), s(190.), w - s(48.), s(156.));
    c.round_rect(sec2, c.ui.surface, s(8.));
    c.text(
        "상단 바 외형 · 시계 형식 · 날씨 지역",
        Rect::new(s(38.), s(198.), s(340.), s(22.)),
        c.ui.accent,
        12.5,
        true,
        0,
    );

    let opacity_opts: Vec<(&str, bool, Edit)> = OPACITY_STEPS
        .iter()
        .enumerate()
        .map(|(i, (label, _))| (*label, v.opacity_idx == i, Edit::SetOpacity(i)))
        .collect();
    draw_segmented_row(
        c,
        &mut hits,
        "상단 바 투명도",
        s(226.),
        w,
        &opacity_opts,
        hover,
    );

    let clock_opts: Vec<(&str, bool, Edit)> = [
        ("전체 (년/월/일 시:분)", 0),
        ("시간만 (시:분)", 1),
        ("날짜만 (년/월/일)", 2),
        ("간략형 (월/일 시:분)", 3),
        ("ISO (YYYY-MM-DD)", 4),
    ]
    .into_iter()
    .map(|(label, i)| (label, v.clock_format_idx == i, Edit::SetClockFormat(i)))
    .collect();
    draw_segmented_row(
        c,
        &mut hits,
        "시계 표시 형식",
        s(264.),
        w,
        &clock_opts,
        hover,
    );

    let city_opts: Vec<(&str, bool, Edit)> = CITIES
        .iter()
        .enumerate()
        .map(|(i, city)| (city.name, v.weather_city == i, Edit::SetCity(i)))
        .collect();
    draw_segmented_row(
        c,
        &mut hits,
        "날씨 측정 지역",
        s(302.),
        w,
        &city_opts,
        hover,
    );

    // -------------------------------------------------------------------------
    // Card 3: 창 관리 및 단축키 기본 실행 프로그램
    // -------------------------------------------------------------------------
    let sec3 = Rect::new(s(24.), s(356.), w - s(48.), s(156.));
    c.round_rect(sec3, c.ui.surface, s(8.));
    c.text(
        "창 관리 및 단축키 기본 실행 프로그램",
        Rect::new(s(38.), s(364.), s(360.), s(22.)),
        c.ui.accent,
        12.5,
        true,
        0,
    );

    let split_opts = [
        (
            "켜짐 · 창 비율에 따라 가로/세로 자동 분할",
            v.auto_split,
            Edit::SetAutoSplit(true),
        ),
        (
            "꺼짐 · 수동 분할 방향만 유지",
            !v.auto_split,
            Edit::SetAutoSplit(false),
        ),
    ];
    draw_segmented_row(
        c,
        &mut hits,
        "자동 분할 (Auto-Split)",
        s(392.),
        w,
        &split_opts,
        hover,
    );

    let term_opts: Vec<(&str, bool, Edit)> = TERMINALS
        .iter()
        .enumerate()
        .map(|(i, (label, _))| (*label, v.terminal_idx == i, Edit::SetTerminal(i)))
        .collect();
    draw_segmented_row(
        c,
        &mut hits,
        "터미널 (Alt+Enter)",
        s(430.),
        w,
        &term_opts,
        hover,
    );

    let brow_opts: Vec<(&str, bool, Edit)> = BROWSERS
        .iter()
        .enumerate()
        .map(|(i, (label, _))| (*label, v.browser_idx == i, Edit::SetBrowser(i)))
        .collect();
    draw_segmented_row(
        c,
        &mut hits,
        "브라우저 (Alt+Shift+Enter)",
        s(468.),
        w,
        &brow_opts,
        hover,
    );

    // -------------------------------------------------------------------------
    // Bottom Bar: 실시간 반영 상태 안내 및 유틸리티 버튼
    // -------------------------------------------------------------------------
    let bottom_y = h - s(54.);
    let footer = if v.status.is_empty() {
        "✓ 모든 설정은 클릭 즉시 상단 바와 GlazeWM에 실시간으로 반영됩니다"
    } else {
        &v.status
    };
    c.text(
        footer,
        Rect::new(s(28.), bottom_y, w - s(530.), s(34.)),
        if v.status.is_empty() {
            c.ui.muted
        } else {
            c.ui.accent
        },
        11.5,
        !v.status.is_empty(),
        0,
    );

    let btn_sheet = Rect::new(w - s(494.), bottom_y, s(126.), s(34.));
    let btn_cfg = Rect::new(w - s(360.), bottom_y, s(134.), s(34.));
    let btn_reload = Rect::new(w - s(218.), bottom_y, s(112.), s(34.));
    let btn_save = Rect::new(w - s(98.), bottom_y, s(74.), s(34.));

    for (label, r, selected, edit) in [
        ("단축키 안내", btn_sheet, false, Edit::OpenSheet),
        ("config.yaml 편집", btn_cfg, false, Edit::OpenConfig),
        ("WM 다시 적용", btn_reload, false, Edit::ReloadGlazeWM),
        ("완료", btn_save, true, Edit::Save),
    ] {
        views::button(
            c,
            &mut hits,
            label,
            r,
            selected,
            hover.is_some_and(|(x, y)| r.contains(x, y)),
            true,
            Action::Settings(edit),
        );
    }

    // Legacy dropdown support (kept for programmatic test compatibility if open_dropdown is set)
    if let Some(SettingsDropdown::Opacity) = v.open_dropdown {
        let anchor = Rect::new(s(208.), s(226.), s(140.), s(30.));
        let options: Vec<views::DropdownOption> = OPACITY_STEPS
            .iter()
            .enumerate()
            .map(|(i, (label, _))| views::DropdownOption::new(*label, v.opacity_idx == i))
            .collect();
        let (items, dismiss) = views::dropdown(c, anchor, &options, hover);
        let mut dropdown_hits: Vec<Hit> = items
            .into_iter()
            .map(|(rect, index)| Hit {
                rect,
                action: Action::Settings(Edit::SetOpacity(index)),
            })
            .collect();
        dropdown_hits.push(Hit {
            rect: dismiss,
            action: Action::Settings(Edit::CloseDropdown),
        });
        dropdown_hits.append(&mut hits);
        hits = dropdown_hits;
    }

    c.frame(Rect::new(0, 0, w, h), c.ui.border);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_dropdown_item_wins_over_dismiss_target() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let settings = BarSettings {
            open_dropdown: Some(SettingsDropdown::Opacity),
            ..Default::default()
        };
        let mut hits = Vec::new();
        crate::graphics::render_png(
            root,
            &output.path().join("settings.png"),
            800,
            600,
            1.,
            crate::graphics::Palette::load(root),
            |canvas| {
                hits = paint(canvas, &settings, None, true);
            },
        )
        .unwrap();
        let first = hits
            .iter()
            .find(|hit| hit.action == Action::Settings(Edit::SetOpacity(0)))
            .expect("opacity menu item hit");
        assert_eq!(
            first.action,
            Action::Settings(Edit::SetOpacity(0)),
            "the dropdown item must take precedence over the dismiss target"
        );
    }

    #[test]
    fn settings_roundtrip_and_config_update() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("config.yaml"),
            "gaps:\n  inner_gap: '16px'\nkeybindings:\n  - commands: ['shell-exec wt']\n    bindings: ['alt+enter', 'win+enter']\n  - commands: ['shell-exec cmd /c start \"\" \"https://www.google.com\"']\n    bindings: ['alt+shift+enter']\n",
        )
        .unwrap();
        let mut s = BarSettings::load(dir.path());
        assert!(s.show_weather);
        assert_eq!(s.targets, [true; 16]);
        s.apply_edit(&Edit::CycleCity);
        s.apply_edit(&Edit::CycleClockFormat);
        s.apply_edit(&Edit::CycleOpacity);
        s.apply_edit(&Edit::CycleTerminal);
        s.apply_edit(&Edit::ToggleAutoSplit);
        s.apply_edit(&Edit::ToggleTarget(4));
        s.save(dir.path()).unwrap();

        let loaded = BarSettings::load(dir.path());
        assert_eq!(loaded.weather_city, 1);
        assert_eq!(loaded.clock_format_idx, 1);
        assert_eq!(loaded.opacity_idx, 1);
        assert_eq!(loaded.terminal_idx, 1);
        assert!(!loaded.auto_split);
        assert!(!loaded.targets[4]);
        assert!(dir.path().join("autotile.disabled").exists());
        let cfg = std::fs::read_to_string(dir.path().join("config.yaml")).unwrap();
        assert!(cfg.contains("shell-exec rio"));
    }
}
