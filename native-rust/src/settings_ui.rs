use crate::{
    graphics::{Canvas, Rect},
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
    ("Windows Terminal (wt)", "shell-exec wt"),
    ("Rio Terminal (rio)", "shell-exec rio"),
    ("PowerShell (pwsh)", "shell-exec pwsh"),
    ("Alacritty", "shell-exec alacritty"),
    ("WezTerm", "shell-exec wezterm-gui"),
    ("Hyper", "shell-exec hyper"),
];

pub const BROWSERS: [(&str, &str); 4] = [
    (
        "기본 브라우저 (Google)",
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
    ("0% (완전 투명)", 0),
];

pub const TOPBAR_WIDGET_IDS: [&str; 9] = [
    "weather",
    "battery",
    "cpu",
    "memory",
    "wifi",
    "tiling",
    "clock",
    "scale",
    "brightness",
];

pub const TOPBAR_WIDGET_NAMES: [&str; 9] = [
    "날씨 정보",
    "배터리 / 전력",
    "CPU 사용률",
    "메모리 사용률",
    "Wi-Fi 네트워크",
    "창 분할 방향 전환",
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
    pub show_brightness: bool,
    #[serde(default = "default_true")]
    pub show_scale: bool,
    #[serde(default = "default_true")]
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
            show_brightness: true,
            show_scale: true,
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
    OpenWallpaper,
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
            "brightness" => self.show_brightness,
            "scale" => self.show_scale,
            "tiling" => true,
            "clock" => true,
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
            "brightness" => self.show_brightness = visible,
            "scale" => self.show_scale = visible,
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
            Edit::ToggleBrightness => self.show_brightness = !self.show_brightness,
            Edit::ToggleScale => self.show_scale = !self.show_scale,
            Edit::ToggleQuickButtons => self.show_quick_buttons = !self.show_quick_buttons,
            Edit::SetWidgetVisible(id, visible) => {
                self.set_widget_visible(id, *visible);
                self.open_dropdown = None;
            }
            Edit::MoveWidget(idx, delta) => {
                let mut order = self.normalized_widget_order();
                let new_idx = (*idx as isize + delta).clamp(0, order.len() as isize - 1) as usize;
                if new_idx != *idx && *idx < order.len() {
                    let item = order.remove(*idx);
                    order.insert(new_idx, item);
                    self.widget_order = order;
                }
            }
            Edit::ResetWidgetOrder => {
                self.widget_order = default_widget_order();
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
                self.weather_city = *idx;
                self.open_dropdown = None;
            }
            Edit::SetClockFormat(idx) => {
                self.clock_format_idx = *idx;
                self.open_dropdown = None;
            }
            Edit::SetOpacity(idx) => {
                self.opacity_idx = *idx;
                self.open_dropdown = None;
            }
            Edit::SetTerminal(idx) => {
                self.terminal_idx = *idx;
                self.open_dropdown = None;
            }
            Edit::SetBrowser(idx) => {
                self.browser_idx = *idx;
                self.open_dropdown = None;
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

fn toggle(c: &mut Canvas, hits: &mut Vec<Hit>, label: &str, r: Rect, checked: bool, edit: Edit) {
    let side = c.p(11.);
    let y = r.y + (r.h - side) / 2;
    let col = if checked { c.ui.accent } else { c.ui.border };
    c.frame(Rect::new(r.x, y, side, side), col);
    if checked {
        let inset = c.p(3.);
        c.fill(
            Rect::new(r.x + inset, y + inset, side - 2 * inset, side - 2 * inset),
            col,
        );
    }
    c.text(
        label,
        Rect::new(r.x + side + c.p(8.), r.y, r.w - side - c.p(8.), r.h),
        c.ui.text,
        12.,
        false,
        0,
    );
    hits.push(Hit {
        rect: r,
        action: Action::Settings(edit),
    });
}

pub fn paint(c: &mut Canvas, v: &BarSettings, hover: Option<(i32, i32)>) -> Vec<Hit> {
    let mut hits = vec![];
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let w = c.width;
    let h = c.height;

    c.text(
        "GlazeBar 환경 설정",
        Rect::new(s(26.), s(16.), s(320.), s(30.)),
        c.ui.text,
        18.,
        true,
        0,
    );
    c.text(
        "상단 바 표시 위젯 및 위치 순서, 날씨 지역, 기본 실행 프로그램, 테마 연동 앱을 설정합니다",
        Rect::new(s(26.), s(46.), w - s(140.), s(22.)),
        c.ui.muted,
        11.5,
        false,
        0,
    );

    let close_rect = Rect::new(w - s(98.), s(18.), s(72.), s(30.));
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

    // Section 1: 상단 바 위젯 및 디스플레이 드롭다운 설정
    let sec1 = Rect::new(s(24.), s(76.), w - s(48.), s(195.));
    c.round_rect(sec1, c.ui.surface, s(8.));
    c.text(
        "상단 바 위젯 표시 설정 및 위치 순서 변경 (드롭다운)",
        Rect::new(s(38.), s(84.), s(380.), s(22.)),
        c.ui.accent,
        12.,
        true,
        0,
    );

    let city_name = CITIES.get(v.weather_city).map(|c| c.name).unwrap_or("서울");
    let clock_name = CLOCK_FORMATS
        .get(v.clock_format_idx)
        .map(|c| c.0)
        .unwrap_or(CLOCK_FORMATS[0].0);
    let opacity_name = OPACITY_STEPS
        .get(v.opacity_idx)
        .map(|s| s.0)
        .unwrap_or(OPACITY_STEPS[0].0);

    let btn_w = s(140.);
    let city_btn = Rect::new(w - s(38.) - btn_w, s(82.), btn_w, s(26.));
    views::button(
        c,
        &mut hits,
        &format!("날씨: {city_name} ▾"),
        city_btn,
        v.open_dropdown == Some(SettingsDropdown::City),
        hover.is_some_and(|(x, y)| city_btn.contains(x, y)),
        true,
        Action::Settings(Edit::ToggleDropdown(SettingsDropdown::City)),
    );

    let clock_btn = Rect::new(city_btn.x - s(8.) - btn_w, s(82.), btn_w, s(26.));
    views::button(
        c,
        &mut hits,
        &format!("시계: {clock_name} ▾"),
        clock_btn,
        v.open_dropdown == Some(SettingsDropdown::ClockFormat),
        hover.is_some_and(|(x, y)| clock_btn.contains(x, y)),
        true,
        Action::Settings(Edit::ToggleDropdown(SettingsDropdown::ClockFormat)),
    );

    let opacity_btn = Rect::new(clock_btn.x - s(8.) - btn_w, s(82.), btn_w, s(26.));
    views::button(
        c,
        &mut hits,
        &format!("투명도: {opacity_name} ▾"),
        opacity_btn,
        v.open_dropdown == Some(SettingsDropdown::Opacity),
        hover.is_some_and(|(x, y)| opacity_btn.contains(x, y)),
        true,
        Action::Settings(Edit::ToggleDropdown(SettingsDropdown::Opacity)),
    );

    // 위젯 리스트 및 드롭다운 (3열 그리드: 날씨, 배터리, CPU, 메모리, Wi-Fi, 밝기, 배율, 빠른실행)
    let order = v.normalized_widget_order();
    let col_w = (w - s(80.)) / 3;
    let mut widget_dropdown_rects = Vec::new();

    for (i, widget_id) in order.iter().enumerate() {
        let row = i / 3;
        let col = i % 3;
        let item_r = Rect::new(
            s(38.) + col as i32 * col_w,
            s(116. + row as f64 * 34.),
            col_w - s(8.),
            s(28.),
        );

        let is_visible = v.is_widget_visible(widget_id);
        let name_idx = TOPBAR_WIDGET_IDS
            .iter()
            .position(|&id| id == widget_id)
            .unwrap_or(0);
        let display_name = TOPBAR_WIDGET_NAMES[name_idx];
        let status_label = if is_visible {
            "표시함 ▾"
        } else {
            "숨김 ▾"
        };
        let full_label = format!("{display_name}: {status_label}");

        let is_open = v.open_dropdown == Some(SettingsDropdown::WidgetVisibility(i));
        let item_hover = hover.is_some_and(|(x, y)| item_r.contains(x, y));

        views::button(
            c,
            &mut hits,
            &full_label,
            item_r,
            is_open || is_visible,
            item_hover,
            true,
            Action::Settings(Edit::ToggleDropdown(SettingsDropdown::WidgetVisibility(i))),
        );

        if is_open {
            widget_dropdown_rects.push((item_r, i, widget_id.clone(), is_visible));
        }
    }

    // Reset order button
    let reset_order_r = Rect::new(w - s(170.), s(232.), s(132.), s(26.));
    views::button(
        c,
        &mut hits,
        "위젯 순서 초기화 ↺",
        reset_order_r,
        false,
        hover.is_some_and(|(x, y)| reset_order_r.contains(x, y)),
        true,
        Action::Settings(Edit::ResetWidgetOrder),
    );

    // Section 2: GlazeWM 동작 및 기본 실행 앱
    let sec2 = Rect::new(s(24.), s(280.), w - s(48.), s(116.));
    c.round_rect(sec2, c.ui.surface, s(8.));
    c.text(
        "창 관리 및 기본 실행 프로그램 (드롭다운)",
        Rect::new(s(38.), s(288.), s(360.), s(22.)),
        c.ui.accent,
        12.,
        true,
        0,
    );

    // Auto-split toggle converted to stylized dropdown selector button
    let autosplit_r = Rect::new(s(38.), s(314.), w - s(76.), s(28.));
    let autosplit_text = if v.auto_split {
        "창 비율 자동 분할 (Auto-Split): [켬 - 비율에 따라 가로/세로 전환] (클릭하여 변경)"
    } else {
        "창 비율 자동 분할 (Auto-Split): [끔 - 수동 분할만 사용] (클릭하여 변경)"
    };
    views::button(
        c,
        &mut hits,
        autosplit_text,
        autosplit_r,
        v.auto_split,
        hover.is_some_and(|(x, y)| autosplit_r.contains(x, y)),
        true,
        Action::Settings(Edit::ToggleAutoSplit),
    );

    let term_label = TERMINALS
        .get(v.terminal_idx)
        .map(|t| t.0)
        .unwrap_or(TERMINALS[0].0);
    let brow_label = BROWSERS
        .get(v.browser_idx)
        .map(|b| b.0)
        .unwrap_or(BROWSERS[0].0);
    let btn_w = (w - s(96.)) / 2;
    let r_term = Rect::new(s(38.), s(350.), btn_w, s(32.));
    views::button(
        c,
        &mut hits,
        &format!("터미널: {term_label} ▾"),
        r_term,
        v.open_dropdown == Some(SettingsDropdown::Terminal),
        hover.is_some_and(|(x, y)| r_term.contains(x, y)),
        true,
        Action::Settings(Edit::ToggleDropdown(SettingsDropdown::Terminal)),
    );

    let r_brow = Rect::new(s(38.) + btn_w + s(20.), s(350.), btn_w, s(32.));
    views::button(
        c,
        &mut hits,
        &format!("브라우저: {brow_label} ▾"),
        r_brow,
        v.open_dropdown == Some(SettingsDropdown::Browser),
        hover.is_some_and(|(x, y)| r_brow.contains(x, y)),
        true,
        Action::Settings(Edit::ToggleDropdown(SettingsDropdown::Browser)),
    );

    // Section 3: 테마 연동 앱 대상 프로그램
    let sec3 = Rect::new(s(24.), s(404.), w - s(48.), s(130.));
    c.round_rect(sec3, c.ui.surface, s(8.));
    c.text(
        "테마 자동 적용 대상 프로그램 (미설치 앱은 자동 건너뜀)",
        Rect::new(s(38.), s(412.), s(440.), s(22.)),
        c.ui.accent,
        12.,
        true,
        0,
    );

    let all_on = v.targets.iter().all(|&b| b);
    let btn_all = Rect::new(w - s(224.), s(410.), s(186.), s(24.));
    views::button(
        c,
        &mut hits,
        if all_on {
            "전체 선택 해제"
        } else {
            "전체 프로그램 선택"
        },
        btn_all,
        false,
        hover.is_some_and(|(x, y)| btn_all.contains(x, y)),
        true,
        Action::Settings(Edit::SelectAllTargets(!all_on)),
    );

    // Render 16 apps in compact grid
    for (i, label) in APP_LABELS.iter().enumerate() {
        let row = i / 4;
        let col = i % 4;
        toggle(
            c,
            &mut hits,
            label,
            Rect::new(
                s(38.) + col as i32 * col_w,
                s(438. + row as f64 * 23.),
                col_w - s(8.),
                s(20.),
            ),
            v.targets[i],
            Edit::ToggleTarget(i),
        );
    }

    // Bottom quick actions
    for (label, x, width, selected, edit) in [
        (
            "테마 · 배경화면 열기",
            24.,
            160.,
            false,
            Edit::OpenWallpaper,
        ),
        ("단축키 안내 보기", 194., 145., false, Edit::OpenSheet),
        ("config.yaml 직접 편집", 349., 165., false, Edit::OpenConfig),
        ("GlazeWM 재적용", 524., 135., false, Edit::ReloadGlazeWM),
        ("저장 완료", 686., 150., true, Edit::Save),
    ] {
        let r = Rect::new(s(x), s(542.), s(width), s(34.));
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

    let footer = if v.status.is_empty() {
        "변경 사항은 즉시 반영되며 [저장 완료] 또는 Enter 키를 누르면 설정 파일에 저장됩니다."
    } else {
        &v.status
    };
    c.text(
        footer,
        Rect::new(s(24.), h - s(32.), w - s(48.), s(22.)),
        c.ui.muted,
        11.5,
        false,
        1,
    );

    // Render Dropdown Menus if active (Overlay with precedence)
    if let Some(dropdown) = v.open_dropdown {
        let mut dropdown_hits = vec![Hit {
            rect: Rect::new(0, 0, w, h),
            action: Action::Settings(Edit::CloseDropdown),
        }];

        match dropdown {
            SettingsDropdown::WidgetVisibility(idx) => {
                if let Some(&(anchor, _, ref wid, is_vis)) =
                    widget_dropdown_rects.iter().find(|(_, i, _, _)| *i == idx)
                {
                    let menu_w = anchor.w.max(s(160.));
                    let item_h = s(26.);
                    let items_count = 4; // [표시함, 숨김, 위로 이동, 아래로 이동]
                    let menu_h = item_h * items_count + s(8.);
                    let menu_x = anchor.x;
                    let menu_y = anchor.y + anchor.h + s(2.);
                    let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                    c.fill(menu_rect, c.ui.surface);
                    c.frame(menu_rect, c.ui.border);

                    let options: [(&str, Edit, bool); 4] = [
                        (
                            "✓ 상단 바에 표시함",
                            Edit::SetWidgetVisible(wid.clone(), true),
                            is_vis,
                        ),
                        (
                            "✕ 상단 바에서 숨김",
                            Edit::SetWidgetVisible(wid.clone(), false),
                            !is_vis,
                        ),
                        ("▲ 왼쪽(앞)으로 위치 이동", Edit::MoveWidget(idx, -1), false),
                        (
                            "▼ 오른쪽(뒤)으로 위치 이동",
                            Edit::MoveWidget(idx, 1),
                            false,
                        ),
                    ];

                    for (opt_idx, (text, action, selected)) in options.into_iter().enumerate() {
                        let item_r = Rect::new(
                            menu_x + s(4.),
                            menu_y + s(4.) + opt_idx as i32 * item_h,
                            menu_w - s(8.),
                            item_h,
                        );
                        let is_h = hover.is_some_and(|(x, y)| item_r.contains(x, y));
                        if selected {
                            c.fill(item_r, c.ui.selected);
                        } else if is_h {
                            c.fill(item_r, c.ui.hover);
                        }
                        c.text(
                            text,
                            Rect::new(item_r.x + s(8.), item_r.y, item_r.w - s(16.), item_r.h),
                            if selected { c.ui.accent } else { c.ui.text },
                            11.5,
                            selected,
                            0,
                        );
                        dropdown_hits.push(Hit {
                            rect: item_r,
                            action: Action::Settings(action),
                        });
                    }
                }
            }
            SettingsDropdown::City => {
                let menu_w = s(160.);
                let item_h = s(24.);
                let menu_h = item_h * CITIES.len() as i32 + s(8.);
                let menu_x = city_btn.x;
                let menu_y = city_btn.y + city_btn.h + s(2.);
                let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                c.fill(menu_rect, c.ui.surface);
                c.frame(menu_rect, c.ui.border);

                for (idx, city) in CITIES.iter().enumerate() {
                    let item_r = Rect::new(
                        menu_x + s(4.),
                        menu_y + s(4.) + idx as i32 * item_h,
                        menu_w - s(8.),
                        item_h,
                    );
                    let is_sel = v.weather_city == idx;
                    let is_h = hover.is_some_and(|(x, y)| item_r.contains(x, y));
                    if is_sel {
                        c.fill(item_r, c.ui.selected);
                    } else if is_h {
                        c.fill(item_r, c.ui.hover);
                    }
                    c.text(
                        city.name,
                        Rect::new(item_r.x + s(8.), item_r.y, item_r.w - s(16.), item_r.h),
                        if is_sel { c.ui.accent } else { c.ui.text },
                        11.5,
                        is_sel,
                        0,
                    );
                    dropdown_hits.push(Hit {
                        rect: item_r,
                        action: Action::Settings(Edit::SetCity(idx)),
                    });
                }
            }
            SettingsDropdown::ClockFormat => {
                let menu_w = s(170.);
                let item_h = s(24.);
                let menu_h = item_h * CLOCK_FORMATS.len() as i32 + s(8.);
                let menu_x = clock_btn.x;
                let menu_y = clock_btn.y + clock_btn.h + s(2.);
                let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                c.fill(menu_rect, c.ui.surface);
                c.frame(menu_rect, c.ui.border);

                for (idx, (label, desc)) in CLOCK_FORMATS.iter().enumerate() {
                    let item_r = Rect::new(
                        menu_x + s(4.),
                        menu_y + s(4.) + idx as i32 * item_h,
                        menu_w - s(8.),
                        item_h,
                    );
                    let is_sel = v.clock_format_idx == idx;
                    let is_h = hover.is_some_and(|(x, y)| item_r.contains(x, y));
                    if is_sel {
                        c.fill(item_r, c.ui.selected);
                    } else if is_h {
                        c.fill(item_r, c.ui.hover);
                    }
                    c.text(
                        &format!("{label} ({desc})"),
                        Rect::new(item_r.x + s(8.), item_r.y, item_r.w - s(16.), item_r.h),
                        if is_sel { c.ui.accent } else { c.ui.text },
                        11.,
                        is_sel,
                        0,
                    );
                    dropdown_hits.push(Hit {
                        rect: item_r,
                        action: Action::Settings(Edit::SetClockFormat(idx)),
                    });
                }
            }
            SettingsDropdown::Opacity => {
                let menu_w = s(150.);
                let item_h = s(24.);
                let menu_h = item_h * OPACITY_STEPS.len() as i32 + s(8.);
                let menu_x = opacity_btn.x;
                let menu_y = opacity_btn.y + opacity_btn.h + s(2.);
                let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                c.fill(menu_rect, c.ui.surface);
                c.frame(menu_rect, c.ui.border);

                for (idx, (label, _)) in OPACITY_STEPS.iter().enumerate() {
                    let item_r = Rect::new(
                        menu_x + s(4.),
                        menu_y + s(4.) + idx as i32 * item_h,
                        menu_w - s(8.),
                        item_h,
                    );
                    let is_sel = v.opacity_idx == idx;
                    let is_h = hover.is_some_and(|(x, y)| item_r.contains(x, y));
                    if is_sel {
                        c.fill(item_r, c.ui.selected);
                    } else if is_h {
                        c.fill(item_r, c.ui.hover);
                    }
                    c.text(
                        label,
                        Rect::new(item_r.x + s(8.), item_r.y, item_r.w - s(16.), item_r.h),
                        if is_sel { c.ui.accent } else { c.ui.text },
                        11.5,
                        is_sel,
                        0,
                    );
                    dropdown_hits.push(Hit {
                        rect: item_r,
                        action: Action::Settings(Edit::SetOpacity(idx)),
                    });
                }
            }
            SettingsDropdown::Terminal => {
                let menu_w = r_term.w;
                let item_h = s(26.);
                let menu_h = item_h * TERMINALS.len() as i32 + s(8.);
                let menu_x = r_term.x;
                let menu_y = r_term.y + r_term.h + s(2.);
                let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                c.fill(menu_rect, c.ui.surface);
                c.frame(menu_rect, c.ui.border);

                for (idx, (label, _)) in TERMINALS.iter().enumerate() {
                    let item_r = Rect::new(
                        menu_x + s(4.),
                        menu_y + s(4.) + idx as i32 * item_h,
                        menu_w - s(8.),
                        item_h,
                    );
                    let is_sel = v.terminal_idx == idx;
                    let is_h = hover.is_some_and(|(x, y)| item_r.contains(x, y));
                    if is_sel {
                        c.fill(item_r, c.ui.selected);
                    } else if is_h {
                        c.fill(item_r, c.ui.hover);
                    }
                    c.text(
                        label,
                        Rect::new(item_r.x + s(8.), item_r.y, item_r.w - s(16.), item_r.h),
                        if is_sel { c.ui.accent } else { c.ui.text },
                        11.5,
                        is_sel,
                        0,
                    );
                    dropdown_hits.push(Hit {
                        rect: item_r,
                        action: Action::Settings(Edit::SetTerminal(idx)),
                    });
                }
            }
            SettingsDropdown::Browser => {
                let menu_w = r_brow.w;
                let item_h = s(26.);
                let menu_h = item_h * BROWSERS.len() as i32 + s(8.);
                let menu_x = r_brow.x;
                let menu_y = r_brow.y + r_brow.h + s(2.);
                let menu_rect = Rect::new(menu_x, menu_y, menu_w, menu_h);

                c.fill(menu_rect, c.ui.surface);
                c.frame(menu_rect, c.ui.border);

                for (idx, (label, _)) in BROWSERS.iter().enumerate() {
                    let item_r = Rect::new(
                        menu_x + s(4.),
                        menu_y + s(4.) + idx as i32 * item_h,
                        menu_w - s(8.),
                        item_h,
                    );
                    let is_sel = v.browser_idx == idx;
                    let is_h = hover.is_some_and(|(x, y)| item_r.contains(x, y));
                    if is_sel {
                        c.fill(item_r, c.ui.selected);
                    } else if is_h {
                        c.fill(item_r, c.ui.hover);
                    }
                    c.text(
                        label,
                        Rect::new(item_r.x + s(8.), item_r.y, item_r.w - s(16.), item_r.h),
                        if is_sel { c.ui.accent } else { c.ui.text },
                        11.5,
                        is_sel,
                        0,
                    );
                    dropdown_hits.push(Hit {
                        rect: item_r,
                        action: Action::Settings(Edit::SetBrowser(idx)),
                    });
                }
            }
        }

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
