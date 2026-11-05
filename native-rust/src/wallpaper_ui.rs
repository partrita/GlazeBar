use crate::graphics::{Canvas, Rect};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

/// Legacy Aether extraction styles. Kept for the engine, cache, and CLI paths,
/// but no longer exposed in the interactive theme picker.
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
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Dropdown {
    Theme,
}

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
    Target(usize),
    PreviewTab(crate::theme_preview_ui::PreviewTab),
    Windows,
    #[allow(dead_code)] // Legacy Aether cache screen; no longer exposed in the UI.
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
        let extra_targets = [
            bar_settings.targets[4],
            bar_settings.targets[5],
            bar_settings.targets[6],
            bar_settings.targets[7],
            bar_settings.targets[8],
            bar_settings.targets[9],
            bar_settings.targets[10],
            bar_settings.targets[11],
            bar_settings.targets[12],
            bar_settings.targets[13],
            bar_settings.targets[14],
            bar_settings.targets[15],
        ];
        Ok(Self {
            operation: 0,
            revision: 0,
            loaded: Default::default(),
            files,
            selected,
            images: HashMap::new(),
            mode: 0,
            light: false,
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
            palette: None,
            status: String::new(),
            open_dropdown: None,
        })
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
        if let Ok(image) = read_thumbnail(&cache, true) {
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
    // Go keeps the normal appearance while a picker operation is busy.
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
fn toggle(
    c: &mut Canvas,
    hits: &mut Vec<Hit>,
    label: &str,
    r: Rect,
    checked: bool,
    action: Action,
    busy: bool,
) {
    let side = c.p(10.);
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
    if !busy {
        hits.push(Hit { rect: r, action });
    }
}
pub fn paint(c: &mut Canvas, p: &Picker, hover: Option<(i32, i32)>) -> Vec<Hit> {
    let mut hits = vec![];
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let w = c.width;
    let h = c.height;

    // Title
    c.text(
        "배경화면",
        Rect::new(s(28.), s(19.), s(90.), s(28.)),
        c.ui.text,
        17.,
        true,
        0,
    );

    // 테마 색상 함께 적용 토글
    toggle(
        c,
        &mut hits,
        "테마 색상 함께 적용",
        Rect::new(s(122.), s(18.), s(156.), s(30.)),
        p.enabled,
        Action::Enabled,
        p.busy,
    );

    // 정의된 컬러 스킴 선택 드롭다운
    let theme_name = SCHEME_NAMES[p.mode.min(SCHEME_NAMES.len() - 1)];
    let theme_rect = Rect::new(s(288.), s(18.), s(170.), s(30.));
    button(
        c,
        &mut hits,
        &format!("테마: {theme_name} ▾"),
        theme_rect,
        p.open_dropdown == Some(Dropdown::Theme),
        Action::ToggleDropdown(Dropdown::Theme),
        hover,
        p.busy,
    );

    // 다크 / 라이트 선택
    button(
        c,
        &mut hits,
        "다크",
        Rect::new(s(472.), s(18.), s(58.), s(30.)),
        !p.light,
        Action::Light(false),
        hover,
        p.busy,
    );
    button(
        c,
        &mut hits,
        "라이트",
        Rect::new(s(534.), s(18.), s(58.), s(30.)),
        p.light,
        Action::Light(true),
        hover,
        p.busy,
    );

    // Top-right action buttons: 온라인 찾기, 폴더 선택, 닫기
    button(
        c,
        &mut hits,
        "온라인 찾기",
        Rect::new(w - s(256.), s(18.), s(94.), s(30.)),
        false,
        Action::Online,
        hover,
        p.busy,
    );
    button(
        c,
        &mut hits,
        "폴더 선택",
        Rect::new(w - s(156.), s(18.), s(88.), s(30.)),
        false,
        Action::Folder,
        hover,
        p.busy,
    );
    // 창닫기 버튼
    button(
        c,
        &mut hits,
        "닫기 ✕",
        Rect::new(w - s(62.), s(18.), s(52.), s(30.)),
        false,
        Action::Close,
        hover,
        false,
    );

    // Smaller, refined preview
    let mw = (f64::from(w) * 0.46) as i32;
    let mh = (f64::from(mw) * (9.0 / 16.0)) as i32;
    let y = s(66.);
    let side_w = s(36.).max((w - mw) / 2 - s(50.));
    let center = Rect::new((w - mw) / 2, y, mw, mh);
    let cards = [
        Rect::new(
            (w - mw) / 2 - side_w - s(16.),
            y + s(14.),
            side_w,
            mh - s(28.),
        ),
        center,
        Rect::new((w - mw) / 2 + mw + s(16.), y + s(14.), side_w, mh - s(28.)),
    ];

    // Navigation buttons (◀, ▶) beside preview
    let prev_rect = Rect::new((w - mw) / 2 - s(38.), y + (mh - s(32.)) / 2, s(30.), s(32.));
    let next_rect = Rect::new(
        (w - mw) / 2 + mw + s(8.),
        y + (mh - s(32.)) / 2,
        s(30.),
        s(32.),
    );
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

    if p.files.is_empty() {
        c.text("사진 폴더를 선택해주세요", center, c.ui.text, 12., false, 1);
    } else {
        for (i, r) in cards.into_iter().enumerate() {
            if p.files.len() == 1 && i != 1 {
                continue;
            }
            let index = (p.selected + i + p.files.len() - 1) % p.files.len();
            c.fill(r, c.ui.surface);
            if let Some(im) = p.images.get(&p.files[index]) {
                crop(c, im, r, i != 1);
            }
            let col = if i == 1 { c.ui.accent } else { c.ui.border };
            let width = if i == 1 { s(3.) } else { 1 };
            for inset in 0..width {
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
        let label = p.files[p.selected]
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy();
        c.text(
            if label == "TranscodedWallpaper" {
                "현재 배경화면"
            } else {
                &label
            },
            Rect::new(s(24.), y + mh + s(8.), w - s(48.), s(22.)),
            c.ui.text,
            12.,
            false,
            1,
        );
    }

    let palette_y = y + mh + s(34.);
    if !p.enabled {
        c.text(
            "배경화면만 변경 (테마 색상 미적용)",
            Rect::new(s(24.), palette_y, w - s(48.), s(18.)),
            c.ui.muted,
            12.,
            false,
            1,
        );
    } else if let Some(palette) = &p.palette {
        let cell = (w - s(48.)) / 16;
        for (i, col) in palette.iter().enumerate() {
            c.fill(
                Rect::new(s(24.) + i as i32 * cell, palette_y, cell + 1, s(18.)),
                crate::graphics::parse(col),
            );
        }
    }

    // 프리뷰 탭 전환 버튼 (전체 / GlazeBar / Terminal / VS Code)
    let preview_tabs = [
        ("전체", crate::theme_preview_ui::PreviewTab::Overview),
        ("GlazeBar", crate::theme_preview_ui::PreviewTab::GlazeBar),
        ("Terminal", crate::theme_preview_ui::PreviewTab::Terminal),
        ("VS Code", crate::theme_preview_ui::PreviewTab::VsCode),
    ];
    let tab_start_x = 24.;
    let tab_w = 78.;
    for (i, &(tab_label, tab_var)) in preview_tabs.iter().enumerate() {
        button(
            c,
            &mut hits,
            tab_label,
            Rect::new(
                s(tab_start_x + (i as f64 * (tab_w + 6.))),
                palette_y + s(26.),
                s(tab_w),
                s(26.),
            ),
            p.preview_tab == tab_var,
            Action::PreviewTab(tab_var),
            hover,
            p.busy,
        );
    }

    // Targets line (동기화 타깃 토글 칩: GlazeBar, Terminal, Codex CLI, VS Code, Windows)
    let bottom_row1_y = h - s(84.);
    for (i, label) in ["GlazeBar", "Terminal", "Codex CLI", "VS Code"]
        .iter()
        .enumerate()
    {
        toggle(
            c,
            &mut hits,
            label,
            Rect::new(s((24 + i * 86) as f64), bottom_row1_y, s(84.), s(30.)),
            p.targets[i],
            Action::Target(i),
            p.busy,
        );
    }
    button(
        c,
        &mut hits,
        if p.windows {
            "Windows · 켬"
        } else {
            "Windows · 끔"
        },
        Rect::new(s(374.), bottom_row1_y, s(94.), s(30.)),
        p.windows,
        Action::Windows,
        hover,
        p.busy,
    );

    // Utility buttons & prominent Apply button
    for (label, right, width, action) in [
        ("테마 비교", 340., 100., Action::Compare),
        ("테마 복원", 236., 96., Action::Restore),
    ] {
        button(
            c,
            &mut hits,
            label,
            Rect::new(w - s(right), bottom_row1_y, s(width), s(30.)),
            false,
            action,
            hover,
            p.busy,
        );
    }

    // Prominent "적용" (Apply) button
    let apply_rect = Rect::new(w - s(124.), bottom_row1_y - s(2.), s(106.), s(34.));
    let apply_over = hover.is_some_and(|(x, y)| apply_rect.contains(x, y));
    let apply_bg = if apply_over {
        crate::graphics::blend(c.ui.accent, 0xFFFFFF, 0.15)
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

    let mut footer = p.status.clone();
    if footer.is_empty() {
        footer = format!(
            "휠 / ← → 선택   ·   Enter 적용   ·   Esc 닫기    {} / {}",
            p.selected + 1,
            p.files.len()
        );
        if p.loaded.len() < p.files.len() {
            footer += &format!(" · 미리보기 {}/{}", p.loaded.len(), p.files.len());
        }
    }
    let memory: usize = p.images.values().map(|im| im.len()).sum();
    let col = if memory >= 256 << 20 {
        footer = format!(
            "미리보기 메모리 {}MB · 이미지가 많습니다. 사용하지 않는 이미지를 정리해주세요.",
            memory >> 20
        );
        c.ui.warning
    } else {
        c.ui.muted
    };
    c.text(
        &footer,
        Rect::new(s(24.), h - s(43.), w - s(48.), s(24.)),
        col,
        12.,
        false,
        1,
    );

    // 정의된 컬러 스킴 메뉴: 항목 히트가 전체 화면 닫기 히트보다 앞선다.
    if let Some(dropdown) = p.open_dropdown {
        match dropdown {
            Dropdown::Theme => {
                let options: Vec<crate::views::DropdownOption> = SCHEME_IDS
                    .iter()
                    .enumerate()
                    .map(|(index, id)| {
                        let mut option =
                            crate::views::DropdownOption::new(SCHEME_NAMES[index], p.mode == index);
                        if let Some(palette) = crate::color_schemes::palette(id, p.light) {
                            option = option.swatch(crate::graphics::parse(&palette[4]));
                        }
                        option
                    })
                    .collect();
                let (items, dismiss) = crate::views::dropdown(c, theme_rect, &options, hover);
                let mut dropdown_hits: Vec<Hit> = items
                    .into_iter()
                    .map(|(rect, index)| Hit {
                        rect,
                        action: Action::SetMode(index),
                    })
                    .collect();
                dropdown_hits.push(Hit {
                    rect: dismiss,
                    action: Action::CloseDropdown,
                });
                dropdown_hits.append(&mut hits);
                hits = dropdown_hits;
            }
        }
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
