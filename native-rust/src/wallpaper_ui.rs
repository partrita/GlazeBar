use crate::{
    graphics::{Canvas, Rect, blend, parse},
    settings_ui::APP_LABELS,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Legacy Aether extraction styles. Kept for the engine, cache, and CLI paths.
pub const MODES: [&str; 6] = [
    "normal",
    "muted",
    "pastel",
    "material",
    "colorful",
    "high-contrast",
];
/// Canonical ids of the predefined color schemes used by every theme picker.
pub const SCHEME_IDS: [&str; 6] = [
    "catppuccin",
    "everforest",
    "rose-pine",
    "monokai",
    "dracula",
    "tokyo-night",
];
/// Display names matching `SCHEME_IDS` order.
pub const SCHEME_NAMES: [&str; 6] = [
    "Catppuccin",
    "Everforest",
    "Rosé Pine",
    "Monokai",
    "Dracula",
    "Tokyo Night",
];

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dropdown {
    Theme,
}

#[allow(dead_code)]
#[derive(Clone, Debug, PartialEq)]
pub enum Action {
    Select(i32),
    Enabled,
    SetMode(usize),
    ToggleDropdown(Dropdown),
    CloseDropdown,
    Light(bool),
    Online,
    Folder,
    PreviewTab(crate::theme_preview_ui::PreviewTab),
    ToggleTarget(usize),
    SelectAllTargets(bool),
    Windows,
    #[allow(dead_code)]
    Precache,
    Compare,
    Restore,
    Apply,
    Close,
}

pub struct Hit {
    pub rect: Rect,
    pub action: Action,
}

#[derive(Clone)]
pub struct Picker {
    pub operation: u64,
    pub revision: u64,
    pub loaded: std::collections::HashSet<PathBuf>,
    pub files: Vec<PathBuf>,
    pub selected: usize,
    pub images: HashMap<PathBuf, Arc<image::RgbaImage>>,
    pub mode: usize,
    pub light: bool,
    pub enabled: bool,
    pub targets: [bool; 4],
    pub extra_targets: [bool; 12],
    pub windows: bool,
    pub options: crate::theme_windows::Options,
    pub preview_tab: crate::theme_preview_ui::PreviewTab,
    pub busy: bool,
    pub palette: Option<crate::theme::Palette>,
    pub status: String,
    pub open_dropdown: Option<Dropdown>,
}

impl Picker {
    pub fn load(root: &Path) -> anyhow::Result<Self> {
        let home = PathBuf::from(
            std::env::var_os("USERPROFILE")
                .ok_or_else(|| anyhow::anyhow!("USERPROFILE missing"))?,
        );
        let system = PathBuf::from(
            std::env::var_os("SystemRoot").ok_or_else(|| anyhow::anyhow!("SystemRoot missing"))?,
        );
        let current = crate::wallpaper::current()?;
        let files = crate::wallpaper::files(
            &crate::wallpaper::folder(root, &home),
            &system.join("Web").join("Wallpaper"),
            &current,
        );
        let key = |p: &Path| {
            p.components()
                .collect::<PathBuf>()
                .to_string_lossy()
                .to_lowercase()
        };
        let selected = files
            .iter()
            .position(|p| key(p) == key(&current))
            .unwrap_or(0);
        let preferences: serde_json::Value =
            std::fs::read(root.join("theme-state/windows-preferences.json"))
                .ok()
                .and_then(|b| serde_json::from_slice(&b).ok())
                .unwrap_or_default();
        let bar_settings = crate::settings_ui::BarSettings::load(root);
        let targets = [
            bar_settings.targets[0],
            bar_settings.targets[1],
            bar_settings.targets[2],
            bar_settings.targets[3],
        ];
        let mut extra_targets = [true; 12];
        extra_targets.copy_from_slice(&bar_settings.targets[4..16]);

        let current_json_path = home.join(".glzr/glazewm/theme-state/current-palette.json");
        let current_state: Option<serde_json::Value> = std::fs::read(&current_json_path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok());
        let light = current_state
            .as_ref()
            .and_then(|v| v["light"].as_bool())
            .unwrap_or(false);
        let mode = current_state
            .as_ref()
            .and_then(|v| v["mode"].as_str())
            .and_then(|m| SCHEME_IDS.iter().position(|&id| id == m))
            .unwrap_or(0);
        let palette = crate::color_schemes::palette(SCHEME_IDS[mode], light);

        Ok(Self {
            operation: 0,
            revision: 0,
            loaded: Default::default(),
            files,
            selected,
            images: HashMap::new(),
            mode,
            light,
            enabled: true,
            targets,
            extra_targets,
            windows: preferences["enabled"].as_bool().unwrap_or(false),
            options: serde_json::from_value(preferences["options"].clone()).unwrap_or(
                crate::theme_windows::Options {
                    mode: true,
                    accent: true,
                    ..Default::default()
                },
            ),
            preview_tab: Default::default(),
            busy: false,
            palette,
            status: String::new(),
            open_dropdown: None,
        })
    }

    pub fn all_targets(&self) -> [bool; 16] {
        let mut out = [true; 16];
        out[..4].copy_from_slice(&self.targets);
        out[4..16].copy_from_slice(&self.extra_targets);
        out
    }

    pub fn toggle_target(&mut self, index: usize) {
        if index < 4 {
            self.targets[index] = !self.targets[index];
        } else if index < 16 {
            self.extra_targets[index - 4] = !self.extra_targets[index - 4];
        }
        self.status.clear();
    }

    pub fn select_all_targets(&mut self, enabled: bool) {
        self.targets = [enabled; 4];
        self.extra_targets = [enabled; 12];
        self.status.clear();
    }

    pub fn select(&mut self, delta: i32) {
        if !self.busy && !self.files.is_empty() {
            self.selected = (self.selected as i64 + i64::from(delta))
                .rem_euclid(self.files.len() as i64) as usize;
            self.status.clear();
        }
    }

    pub fn neighbors(&self) -> Vec<PathBuf> {
        if self.files.is_empty() {
            return vec![];
        }
        let mut paths = vec![];
        for d in [0, -1, 1] {
            let i = (self.selected as i64 + d).rem_euclid(self.files.len() as i64) as usize;
            if !paths.contains(&self.files[i]) {
                paths.push(self.files[i].clone());
            }
        }
        paths
    }
}

pub fn thumbnail(path: &Path) -> anyhow::Result<image::RgbaImage> {
    thumbnail_in(path, crate::ui_cache::directory("thumbnail").as_deref())
}

pub(crate) fn thumbnail_in(path: &Path, dir: Option<&Path>) -> anyhow::Result<image::RgbaImage> {
    let key = crate::ui_cache::thumbnail_key(path);
    let cache = dir
        .zip(key.as_deref())
        .map(|(dir, key)| dir.join(format!("{key}.jpg")));
    if let Some(cache) = &cache {
        if let Ok(image) = read_thumbnail(cache, true) {
            return Ok(image);
        }
    }
    let image = read_thumbnail(path, false)?;
    if key == crate::ui_cache::thumbnail_key(path) {
        if let Some(cache) = cache {
            let save = || -> anyhow::Result<()> {
                let rgb: Vec<u8> = image
                    .pixels()
                    .flat_map(|p| {
                        [p[0], p[1], p[2]].map(|v| (u16::from(v) * u16::from(p[3]) / 255) as u8)
                    })
                    .collect();
                let mut bytes = Vec::new();
                image::codecs::jpeg::JpegEncoder::new_with_quality(&mut bytes, 90).encode(
                    &rgb,
                    image.width(),
                    image.height(),
                    image::ExtendedColorType::Rgb8,
                )?;
                crate::theme_transaction::atomic_write(&cache, &bytes)
            };
            if let Err(e) = save() {
                eprintln!("[thumbnail cache] write failed: {e}");
            }
        }
    }
    Ok(image)
}

fn read_thumbnail(path: &Path, cached: bool) -> anyhow::Result<image::RgbaImage> {
    use image::ImageDecoder;
    let decoder = image::ImageReader::open(path)?
        .with_guessed_format()?
        .into_decoder()?;
    let (w, h) = decoder.dimensions();
    anyhow::ensure!(
        !cached || (w <= 640 && h <= 640),
        "Invalid thumbnail cache dimensions"
    );
    anyhow::ensure!(
        w > 0 && h > 0 && u64::from(w) * u64::from(h) <= 80_000_000,
        "Image too large"
    );
    let im = image::DynamicImage::from_decoder(decoder)?.into_rgba8();
    let ratio = (640. / f64::from(w.max(h))).min(1.);
    Ok(image::imageops::resize(
        &im,
        (f64::from(w) * ratio).max(1.) as u32,
        (f64::from(h) * ratio).max(1.) as u32,
        image::imageops::FilterType::CatmullRom,
    ))
}

fn crop(c: &Canvas, im: &image::RgbaImage, r: Rect, dim: bool) {
    let (w, h) = im.dimensions();
    let ratio = (f64::from(r.w) / f64::from(w)).max(f64::from(r.h) / f64::from(h));
    let cw = (f64::from(r.w) / ratio).max(1.) as u32;
    let ch = (f64::from(r.h) / ratio).max(1.) as u32;
    let mut bgra = Vec::with_capacity((cw * ch * 4) as usize);
    for y in (h - ch) / 2..(h - ch) / 2 + ch {
        for x in (w - cw) / 2..(w - cw) / 2 + cw {
            let p = im.get_pixel(x, y).0;
            for k in [2, 1, 0] {
                let v = u32::from(p[k]) * u32::from(p[3]) / 255;
                let v = if dim { v * 155 / 255 } else { v };
                let background = (c.ui.bg >> (k * 8)) & 255;
                bgra.push((v + background * (255 - u32::from(p[3])) / 255) as u8);
            }
            bgra.push(p[3]);
        }
    }
    unsafe {
        use windows_sys::Win32::Graphics::Gdi::{HALFTONE, SetStretchBltMode};
        let previous = SetStretchBltMode(c.dc, HALFTONE);
        c.bitmap(r, cw as i32, ch as i32, &bgra);
        SetStretchBltMode(c.dc, previous);
    }
}

fn button(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    text: &str,
    r: Rect,
    selected: bool,
    action: Action,
    hover: Option<(i32, i32)>,
    busy: bool,
) {
    let over = hover.is_some_and(|(x, y)| r.contains(x, y));
    crate::views::button(
        c,
        &mut vec![],
        text,
        r,
        selected,
        over,
        true,
        crate::views::Action::Close,
    );
    if !busy {
        hits.push(Hit { rect: r, action });
    }
}

fn draw_scheme_card(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    r: Rect,
    index: usize,
    light: bool,
    selected: bool,
    hover: Option<(i32, i32)>,
    busy: bool,
) {
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let id = SCHEME_IDS[index];
    let name = SCHEME_NAMES[index];
    let pal = crate::color_schemes::palette(id, light);
    let over = hover.is_some_and(|(x, y)| r.contains(x, y));

    let card_bg = pal.as_ref().map(|p| parse(&p[0])).unwrap_or(c.ui.bg);
    let card_fg = pal.as_ref().map(|p| parse(&p[15])).unwrap_or(c.ui.text);

    c.fill(
        r,
        if over {
            blend(card_bg, 0xFFFFFF, if light { 0.06 } else { 0.10 })
        } else {
            card_bg
        },
    );

    let border_col = if selected {
        c.ui.accent
    } else if over {
        c.ui.text
    } else {
        c.ui.border
    };
    let border_w = if selected { s(2.).max(2) } else { 1 };
    for inset in 0..border_w {
        c.frame(
            Rect::new(r.x + inset, r.y + inset, r.w - 2 * inset, r.h - 2 * inset),
            border_col,
        );
    }

    let title = if selected {
        format!("✓ {name}")
    } else {
        name.to_string()
    };
    c.text(
        &title,
        Rect::new(r.x + s(8.), r.y + s(6.), r.w - s(16.), s(20.)),
        if selected { c.ui.accent } else { card_fg },
        11.5,
        selected,
        1,
    );

    if let Some(p) = pal {
        let swatch_cols = [&p[1], &p[2], &p[3], &p[4], &p[5], &p[6]];
        let sw_gap = s(4.);
        let total_w = r.w - s(20.);
        let sw_w = (total_w - sw_gap * 5) / 6;
        let sw_h = s(14.);
        let sw_y = r.y + r.h - sw_h - s(8.);
        let start_x = r.x + (r.w - (sw_w * 6 + sw_gap * 5)) / 2;
        for (k, hex) in swatch_cols.into_iter().enumerate() {
            let sr = Rect::new(start_x + k as i32 * (sw_w + sw_gap), sw_y, sw_w, sw_h);
            c.fill(sr, parse(hex));
        }
    }

    if !busy {
        hits.push(Hit {
            rect: r,
            action: Action::SetMode(index),
        });
    }
}

fn draw_app_pill(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    r: Rect,
    label: &str,
    checked: bool,
    index: usize,
    hover: Option<(i32, i32)>,
    busy: bool,
) {
    let over = hover.is_some_and(|(x, y)| r.contains(x, y));
    let bg = if checked {
        if over {
            blend(c.ui.selected, c.ui.accent, 0.25)
        } else {
            blend(c.ui.surface, c.ui.accent, 0.16)
        }
    } else if over {
        c.ui.hover
    } else {
        c.ui.bg
    };
    c.fill(r, bg);
    c.frame(
        r,
        if checked {
            blend(c.ui.border, c.ui.accent, 0.6)
        } else {
            c.ui.border
        },
    );
    let text = if checked {
        format!("✓ {label}")
    } else {
        label.to_string()
    };
    c.text(
        &text,
        r,
        if checked { c.ui.text } else { c.ui.muted },
        11.,
        checked,
        1,
    );
    if !busy {
        hits.push(Hit {
            rect: r,
            action: Action::ToggleTarget(index),
        });
    }
}

/// `chrome` is false when embedded in the settings hub, which supplies the
/// window close button in its shared tab strip.
pub fn paint(c: &mut Canvas, p: &Picker, hover: Option<(i32, i32)>, chrome: bool) -> Vec<Hit> {
    let mut hits = vec![];
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let w = c.width;
    let h = c.height;

    if chrome {
        button(
            c,
            &mut hits,
            "닫기 ✕",
            Rect::new(w - s(76.), s(12.), s(52.), s(28.)),
            false,
            Action::Close,
            hover,
            false,
        );
    }

    // =========================================================================
    // Card 1: 배경화면 탐색 및 미리보기
    // =========================================================================
    let sec1 = Rect::new(s(24.), s(12.), w - s(48.), s(248.));
    c.round_rect(sec1, c.ui.surface, s(8.));

    c.text(
        "배경화면 선택",
        Rect::new(s(38.), s(20.), s(120.), s(28.)),
        c.ui.accent,
        12.5,
        true,
        0,
    );

    if !p.files.is_empty() {
        let raw_name = p.files[p.selected]
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        let display_name = if raw_name == "TranscodedWallpaper" {
            "현재 배경화면"
        } else {
            &raw_name
        };
        c.text(
            &format!(
                "{} / {}  ·  {}",
                p.selected + 1,
                p.files.len(),
                display_name
            ),
            Rect::new(s(154.), s(20.), s(360.), s(28.)),
            c.ui.text,
            11.5,
            false,
            0,
        );
    }

    // Top-right controls inside Card 1
    let sync_toggle_r = Rect::new(w - s(450.), s(20.), s(188.), s(28.));
    button(
        c,
        &mut hits,
        if p.enabled {
            "✓ 테마 색상 함께 적용"
        } else {
            "배경화면만 변경 (테마 유지)"
        },
        sync_toggle_r,
        p.enabled,
        Action::Enabled,
        hover,
        p.busy,
    );

    button(
        c,
        &mut hits,
        "온라인 배경화면 (Wallhaven)",
        Rect::new(w - s(254.), s(20.), s(134.), s(28.)),
        false,
        Action::Online,
        hover,
        p.busy,
    );
    button(
        c,
        &mut hits,
        "폴더 선택…",
        Rect::new(w - s(112.), s(20.), s(76.), s(28.)),
        false,
        Action::Folder,
        hover,
        p.busy,
    );

    // 3-Card Wallpaper Carousel
    let mh = s(186.);
    let mw = (f64::from(mh) * (16.0 / 9.0)) as i32;
    let y = s(58.);
    let side_w = s(36.).max((w - mw) / 2 - s(82.));
    let center = Rect::new((w - mw) / 2, y, mw, mh);
    let cards = [
        Rect::new(
            (w - mw) / 2 - side_w - s(14.),
            y + s(12.),
            side_w,
            mh - s(24.),
        ),
        center,
        Rect::new((w - mw) / 2 + mw + s(14.), y + s(12.), side_w, mh - s(24.)),
    ];

    let prev_rect = Rect::new(s(36.), y + (mh - s(36.)) / 2, s(34.), s(36.));
    let next_rect = Rect::new(w - s(70.), y + (mh - s(36.)) / 2, s(34.), s(36.));

    if p.files.is_empty() {
        c.fill(center, c.ui.bg);
        c.frame(center, c.ui.border);
        c.text(
            "배경화면 폴더에 이미지가 없습니다. 우측 상단 [폴더 선택…]을 눌러주세요.",
            center,
            c.ui.muted,
            12.,
            false,
            1,
        );
    } else {
        for (i, r) in cards.into_iter().enumerate() {
            if p.files.len() == 1 && i != 1 {
                continue;
            }
            let index = (p.selected + i + p.files.len() - 1) % p.files.len();
            c.fill(r, c.ui.bg);
            if let Some(im) = p.images.get(&p.files[index]) {
                crop(c, im, r, i != 1);
            } else {
                c.text("불러오는 중…", r, c.ui.muted, 11., false, 1);
            }
            let col = if i == 1 { c.ui.accent } else { c.ui.border };
            let bw = if i == 1 { s(2.).max(2) } else { 1 };
            for inset in 0..bw {
                c.frame(
                    Rect::new(r.x + inset, r.y + inset, r.w - 2 * inset, r.h - 2 * inset),
                    col,
                );
            }
            if i != 1 && !p.busy {
                hits.push(Hit {
                    rect: r,
                    action: Action::Select(i as i32 - 1),
                });
            }
        }
    }

    // Draw ◀ ▶ buttons after side cards so their hits take priority
    button(
        c,
        &mut hits,
        "◀",
        prev_rect,
        false,
        Action::Select(-1),
        hover,
        p.busy || p.files.is_empty(),
    );
    button(
        c,
        &mut hits,
        "▶",
        next_rect,
        false,
        Action::Select(1),
        hover,
        p.busy || p.files.is_empty(),
    );

    // =========================================================================
    // Card 2: 컬러 스킴 프리셋 (6종 원클릭 카드) 및 16색 팔레트
    // =========================================================================
    let sec2 = Rect::new(s(24.), s(268.), w - s(48.), s(146.));
    c.round_rect(sec2, c.ui.surface, s(8.));

    c.text(
        "컬러 스킴 프리셋",
        Rect::new(s(38.), s(276.), s(150.), s(26.)),
        c.ui.accent,
        12.5,
        true,
        0,
    );
    c.text(
        if p.enabled {
            "원하는 스킴 카드를 클릭하면 하단 16색 팔레트가 즉시 전환됩니다"
        } else {
            "현재 '배경화면만 변경' 상태입니다 (테마 색상을 적용하려면 상단 옵션을 켜세요)"
        },
        Rect::new(s(182.), s(276.), s(520.), s(26.)),
        c.ui.muted,
        11.,
        false,
        0,
    );

    button(
        c,
        &mut hits,
        "다크 모드 (Dark)",
        Rect::new(w - s(248.), s(275.), s(102.), s(26.)),
        !p.light,
        Action::Light(false),
        hover,
        p.busy,
    );
    button(
        c,
        &mut hits,
        "라이트 모드 (Light)",
        Rect::new(w - s(140.), s(275.), s(104.), s(26.)),
        p.light,
        Action::Light(true),
        hover,
        p.busy,
    );

    // 6 Visual Scheme Cards
    let cards_w = w - s(76.);
    let card_gap = s(8.);
    let scheme_w = (cards_w - card_gap * 5) / 6;
    let scheme_h = s(54.);
    let scheme_y = s(308.);
    let active_mode = p.mode.min(SCHEME_IDS.len() - 1);

    for i in 0..SCHEME_IDS.len() {
        let r = Rect::new(
            s(38.) + i as i32 * (scheme_w + card_gap),
            scheme_y,
            scheme_w,
            scheme_h,
        );
        draw_scheme_card(c, &mut hits, r, i, p.light, active_mode == i, hover, p.busy);
    }

    // 16-Color ANSI Palette Bar
    let palette_y = s(370.);
    let active_palette = crate::color_schemes::palette(SCHEME_IDS[active_mode], p.light)
        .or_else(|| p.palette.clone());
    if let Some(palette) = &active_palette {
        let bar_w = w - s(76.);
        let cell = bar_w / 16;
        for (i, col) in palette.iter().enumerate() {
            let cx = s(38.) + i as i32 * cell;
            let cw = if i == 15 {
                s(38.) + bar_w - cx
            } else {
                cell + 1
            };
            c.fill(Rect::new(cx, palette_y, cw, s(32.)), parse(col));
        }
        c.frame(Rect::new(s(38.), palette_y, bar_w, s(32.)), c.ui.border);
    }

    // =========================================================================
    // Card 3: 테마 동기화 대상 프로그램 (16개 앱 원클릭 토글 + Windows 연동)
    // =========================================================================
    let sec3 = Rect::new(s(24.), s(422.), w - s(48.), s(118.));
    c.round_rect(sec3, c.ui.surface, s(8.));

    c.text(
        "테마 동기화 대상 프로그램",
        Rect::new(s(38.), s(430.), s(200.), s(24.)),
        c.ui.accent,
        12.5,
        true,
        0,
    );
    c.text(
        "클릭하여 개별 앱 적용 여부를 선택하세요 (미설치 앱은 자동으로 건너뜁니다)",
        Rect::new(s(230.), s(430.), s(420.), s(24.)),
        c.ui.muted,
        11.,
        false,
        0,
    );

    let all_targets = p.all_targets();
    let all_on = all_targets.iter().all(|&b| b);

    button(
        c,
        &mut hits,
        if p.windows {
            "Windows 시스템 · 켬"
        } else {
            "Windows 시스템 · 끔"
        },
        Rect::new(w - s(278.), s(428.), s(126.), s(26.)),
        p.windows,
        Action::Windows,
        hover,
        p.busy,
    );
    button(
        c,
        &mut hits,
        if all_on {
            "전체 선택 해제"
        } else {
            "전체 프로그램 선택"
        },
        Rect::new(w - s(146.), s(428.), s(110.), s(26.)),
        false,
        Action::SelectAllTargets(!all_on),
        hover,
        p.busy,
    );

    let pill_grid_w = w - s(76.);
    let pill_gap = s(6.);
    let pill_w = (pill_grid_w - pill_gap * 7) / 8;
    let pill_h = s(28.);
    for (i, label) in APP_LABELS.iter().enumerate() {
        let row = i / 8;
        let col = i % 8;
        let r = Rect::new(
            s(38.) + col as i32 * (pill_w + pill_gap),
            s(462.) + row as i32 * (pill_h + s(6.)),
            pill_w,
            pill_h,
        );
        draw_app_pill(c, &mut hits, r, label, all_targets[i], i, hover, p.busy);
    }

    // =========================================================================
    // Bottom Bar: 상태 안내 및 적용 버튼
    // =========================================================================
    let bottom_y = h - s(54.);
    let mut footer = p.status.clone();
    if footer.is_empty() {
        footer = "마우스 휠 / ← → 키로 배경화면 탐색 · 스킴 및 대상 앱 선택 후 [적용 (Enter)] 클릭"
            .into();
    }
    let memory: usize = p.images.values().map(|im| im.len()).sum();
    let col = if memory >= 256 << 20 {
        footer = format!(
            "미리보기 메모리 {}MB · 이미지가 많습니다. 사용하지 않는 이미지를 정리해주세요.",
            memory >> 20
        );
        c.ui.warning
    } else if !p.status.is_empty() {
        c.ui.accent
    } else {
        c.ui.muted
    };
    c.text(
        &footer,
        Rect::new(s(28.), bottom_y, w - s(400.), s(34.)),
        col,
        11.5,
        !p.status.is_empty(),
        0,
    );

    button(
        c,
        &mut hits,
        "스타일 비교",
        Rect::new(w - s(356.), bottom_y, s(96.), s(34.)),
        false,
        Action::Compare,
        hover,
        p.busy,
    );
    button(
        c,
        &mut hits,
        "이전 테마 복원",
        Rect::new(w - s(252.), bottom_y, s(104.), s(34.)),
        false,
        Action::Restore,
        hover,
        p.busy,
    );

    let apply_rect = Rect::new(w - s(140.), bottom_y, s(116.), s(34.));
    let apply_over = hover.is_some_and(|(x, y)| apply_rect.contains(x, y));
    let apply_bg = if apply_over {
        blend(c.ui.accent, 0xFFFFFF, 0.15)
    } else {
        c.ui.accent
    };
    c.fill(apply_rect, apply_bg);
    c.frame(apply_rect, c.ui.border);
    c.text("적용 (Enter)", apply_rect, 0x000000, 12.5, true, 1);
    if !p.busy {
        hits.push(Hit {
            rect: apply_rect,
            action: Action::Apply,
        });
    }

    c.frame(Rect::new(0, 0, w, h), c.ui.border);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jpeg_cache_is_written_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("photo.png");
        image::RgbaImage::from_pixel(800, 400, image::Rgba([100, 80, 60, 128]))
            .save(&source)
            .unwrap();
        let fresh = thumbnail_in(&source, Some(dir.path())).unwrap();
        assert_eq!(fresh.dimensions(), (640, 320));
        let cache = dir.path().join(format!(
            "{}.jpg",
            crate::ui_cache::thumbnail_key(&source).unwrap()
        ));
        let bytes = std::fs::read(&cache).unwrap();
        assert_eq!(&bytes[..2], &[255, 216]);
        let stored = image::open(&cache).unwrap().into_rgba8();
        assert_eq!(thumbnail_in(&source, Some(dir.path())).unwrap(), stored);
        assert_eq!(std::fs::read(cache).unwrap(), bytes);
        let pixel = stored.get_pixel(100, 100);
        assert!(
            pixel[0].abs_diff(50) <= 3 && pixel[1].abs_diff(40) <= 3 && pixel[2].abs_diff(30) <= 3
        );
    }
    #[test]
    fn navigation_wraps_and_busy_preserves_selection() {
        let mut p = Picker {
            operation: 0,
            revision: 0,
            loaded: Default::default(),
            files: vec!["a".into(), "b".into(), "c".into()],
            selected: 0,
            images: Default::default(),
            mode: 0,
            light: false,
            enabled: true,
            targets: [true; 4],
            extra_targets: [true; 12],
            windows: false,
            options: Default::default(),
            busy: false,
            palette: None,
            status: String::new(),
            open_dropdown: None,
            preview_tab: Default::default(),
        };
        p.select(-1);
        assert_eq!(p.selected, 2);
        assert_eq!(
            p.neighbors(),
            vec![PathBuf::from("c"), "b".into(), "a".into()]
        );
        p.select(4);
        assert_eq!(p.selected, 0);
        p.busy = true;
        p.select(1);
        assert_eq!(p.selected, 0);
        p.busy = false;
        p.files.truncate(1);
        assert_eq!(p.neighbors().len(), 1);
        p.files.clear();
        p.select(-1);
        assert!(p.neighbors().is_empty());
    }
}
