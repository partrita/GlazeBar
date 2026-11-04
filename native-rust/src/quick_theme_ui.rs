use crate::{
    color_schemes::SCHEMES,
    graphics::{Canvas, Rect, blend, parse},
    views::{Action, Hit},
};

#[derive(Clone, Debug, PartialEq)]
pub struct QuickTheme {
    pub selected_scheme: String,
    pub light: bool,
}

impl Default for QuickTheme {
    fn default() -> Self {
        Self {
            selected_scheme: "catppuccin".into(),
            light: false,
        }
    }
}

pub fn paint(c: &mut Canvas, state: &QuickTheme, hover: Option<(i32, i32)>) -> Vec<Hit> {
    let mut hits = vec![];
    let p = |v: f64| c.p(v);
    let w = c.width;
    let h = c.height;

    // 배경 및 테두리
    c.fill(Rect::new(0, 0, w, h), c.ui.surface);
    c.frame(Rect::new(0, 0, w, h), c.ui.border);

    // 헤더 타이틀
    c.text(
        "빠른 테마 전환",
        Rect::new(p(16.), p(14.), p(110.), p(24.)),
        c.ui.text,
        13.5,
        true,
        0,
    );

    // 라이트 / 다크 토글 버튼
    let dark_rect = Rect::new(w - p(84.), p(12.), p(34.), p(24.));
    let light_rect = Rect::new(w - p(46.), p(12.), p(34.), p(24.));

    for (is_light, rect, label) in [(false, dark_rect, "🌙"), (true, light_rect, "☀️")] {
        let is_selected = state.light == is_light;
        let is_hover = hover.is_some_and(|(x, y)| rect.contains(x, y));
        let bg = if is_selected {
            c.ui.accent
        } else if is_hover {
            blend(c.ui.surface, c.ui.text, 0.1)
        } else {
            blend(c.ui.surface, c.ui.text, 0.04)
        };
        c.fill(rect, bg);
        c.frame(rect, if is_selected { c.ui.accent } else { c.ui.border });
        c.text(label, rect, if is_selected { 0xffffffff } else { c.ui.text }, 11., is_selected, 1);
        hits.push(Hit {
            rect,
            action: Action::QuickThemeLight(is_light),
        });
    }

    // 구분선
    c.fill(Rect::new(p(14.), p(44.), w - p(28.), 1), c.ui.border);

    // 6대 프리셋 리스트
    let item_h = p(38.);
    let start_y = p(50.);

    for (i, scheme) in SCHEMES.iter().enumerate() {
        let item_y = start_y + (i as i32 * item_h);
        let item_rect = Rect::new(p(10.), item_y, w - p(20.), item_h - p(4.));
        let is_active = state.selected_scheme == scheme.id;
        let is_hover = hover.is_some_and(|(x, y)| item_rect.contains(x, y));

        let bg = if is_active {
            blend(c.ui.surface, c.ui.accent, 0.15)
        } else if is_hover {
            blend(c.ui.surface, c.ui.text, 0.08)
        } else {
            c.ui.surface
        };
        c.fill(item_rect, bg);
        if is_active {
            c.frame(item_rect, blend(c.ui.accent, c.ui.border, 0.3));
        }

        // 테마 4색 칩 (배경, 액센트1, 액센트2, 액센트3)
        let palette = if state.light { scheme.light } else { scheme.dark };
        let chip_colors = [parse(palette[0]), parse(palette[4]), parse(palette[2]), parse(palette[5])];
        let chip_size = p(10.);
        let chip_spacing = p(3.);
        let chip_start_x = p(18.);
        let chip_y = item_y + (item_h - p(4.) - chip_size) / 2;

        for (chip_idx, &col) in chip_colors.iter().enumerate() {
            let chip_rect = Rect::new(chip_start_x + chip_idx as i32 * (chip_size + chip_spacing), chip_y, chip_size, chip_size);
            c.fill(chip_rect, col);
            c.frame(chip_rect, blend(col, c.ui.border, 0.5));
        }

        // 테마 이름
        let text_rect = Rect::new(chip_start_x + p(56.), item_y + p(7.), p(110.), p(20.));
        c.text(
            scheme.name,
            text_rect,
            if is_active { c.ui.accent } else { c.ui.text },
            12.,
            is_active,
            0,
        );

        // 액티브 체크 표시
        if is_active {
            c.text(
                "✓",
                Rect::new(w - p(36.), item_y + p(7.), p(20.), p(20.)),
                c.ui.accent,
                12.,
                true,
                1,
            );
        }

        hits.push(Hit {
            rect: item_rect,
            action: Action::QuickThemeSelect(scheme.id.to_string()),
        });
    }

    // 하단 구분선 및 전체 설정 열기
    let footer_y = h - p(40.);
    c.fill(Rect::new(p(14.), footer_y - p(4.), w - p(28.), 1), c.ui.border);

    let detail_rect = Rect::new(p(10.), footer_y, w - p(20.), p(32.));
    let detail_hover = hover.is_some_and(|(x, y)| detail_rect.contains(x, y));
    c.fill(detail_rect, if detail_hover { blend(c.ui.surface, c.ui.text, 0.08) } else { c.ui.surface });
    c.text(
        "배경화면 및 전체 테마 설정 ↗",
        detail_rect,
        if detail_hover { c.ui.accent } else { c.ui.muted },
        11.5,
        false,
        1,
    );

    hits.push(Hit {
        rect: detail_rect,
        action: Action::Wallpaper,
    });

    hits
}
