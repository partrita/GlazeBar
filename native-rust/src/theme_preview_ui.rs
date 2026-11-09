use crate::graphics::{Canvas, Rect, blend, parse};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PreviewTab {
    #[default]
    Overview,
    GlazeBar,
    Terminal,
    VsCode,
}

#[derive(Clone, PartialEq)]
pub struct Preview {
    pub light: bool,
    pub palette: Option<crate::theme::Palette>,
    pub tab: PreviewTab,
}

fn line(c: &mut Canvas, text: &str, col: u32, x: f64, y: f64) {
    c.text(
        text,
        Rect::new(c.p(x), c.p(y), c.width - c.p(x + 16.), c.p(17.)),
        col,
        12.,
        false,
        0,
    );
}

fn tokens(c: &mut Canvas, a: &[u32; 16], y: f64, parts: &[(&str, usize)]) {
    let mut x = c.p(30.);
    for &(text, slot) in parts {
        let width = c.measure(text, 12., false);
        c.text(
            text,
            Rect::new(x, c.p(y), width, c.p(17.)),
            a[slot],
            12.,
            false,
            0,
        );
        x += width;
    }
}

pub fn paint(c: &mut Canvas, p: &Preview) {
    c.text(
        "테마 미리보기",
        Rect::new(c.p(24.), c.p(18.), c.p(200.), c.p(30.)),
        c.ui.text,
        17.,
        true,
        0,
    );
    c.text(
        if p.light { "라이트" } else { "다크" },
        Rect::new(c.width - c.p(84.), c.p(24.), c.p(64.), c.p(24.)),
        c.ui.muted,
        12.,
        false,
        2,
    );
    c.frame(Rect::new(0, 0, c.width, c.height), c.ui.border);

    // 탭 헤더 영역
    let tabs = [
        ("전체", PreviewTab::Overview),
        ("GlazeBar", PreviewTab::GlazeBar),
        ("Terminal", PreviewTab::Terminal),
        ("VS Code", PreviewTab::VsCode),
    ];
    let tab_w = (c.width - c.p(48.)) / tabs.len() as i32;
    for (idx, &(label, tab_variant)) in tabs.iter().enumerate() {
        let tab_rect = Rect::new(
            c.p(24.) + idx as i32 * tab_w,
            c.p(52.),
            tab_w - c.p(4.),
            c.p(24.),
        );
        let is_active = p.tab == tab_variant;
        c.fill(tab_rect, if is_active { c.ui.accent } else { c.ui.surface });
        c.text(
            label,
            tab_rect,
            if is_active { 0xffffffff } else { c.ui.muted },
            11.,
            is_active,
            1,
        );
    }

    let Some(palette) = &p.palette else {
        c.text(
            "테마 색상이 설정되지 않았습니다",
            Rect::new(0, c.p(80.), c.width, c.height - c.p(80.)),
            c.ui.text,
            12.,
            false,
            1,
        );
        return;
    };
    let a: [u32; 16] = std::array::from_fn(|i| parse(&palette[i]));

    // 16색 팔레트 바 (상단 고정 미니 바)
    let cell = (c.width - c.p(48.)) / 16;
    for (i, col) in a.iter().enumerate() {
        c.fill(
            Rect::new(c.p(24.) + i as i32 * cell, c.p(84.), cell + 1, c.p(14.)),
            *col,
        );
    }

    match p.tab {
        PreviewTab::Overview => {
            line(c, "GlazeBar", c.ui.muted, 24., 112.);
            c.fill(
                Rect::new(c.p(20.), c.p(137.), c.width - c.p(40.), c.p(30.)),
                a[0],
            );
            c.fill(
                Rect::new(c.p(28.), c.p(142.), c.p(24.), c.p(20.)),
                blend(a[0], a[4], 0.28),
            );
            for (text, slot, x) in [
                ("1", 15, 36.),
                ("2", 7, 65.),
                ("3", 7, 94.),
                ("10:30", 15, 155.),
                ("5.2 W", 2, 270.),
            ] {
                line(c, text, a[slot], x, 145.);
            }
            for (label, y, height) in [
                ("Terminal", 185., 75.),
                ("Codex CLI", 285., 125.),
                ("VS Code", 430., 118.),
            ] {
                line(c, label, c.ui.muted, 24., y);
                c.fill(
                    Rect::new(
                        c.p(20.),
                        c.p(y + 25.),
                        c.width - c.p(40.),
                        c.p(height - 25.),
                    ),
                    a[0],
                );
            }
            line(c, "PS  ~", a[6], 30., 220.);
            line(c, "> Get-ChildItem", a[15], 30., 242.);
            tokens(
                c,
                &a,
                321.,
                &[("function ", 5), ("preview", 4), ("() {", 15)],
            );
            tokens(
                c,
                &a,
                345.,
                &[("  return ", 5), ("\"wallpaper\"", 2), (";", 15)],
            );
            line(c, "}", a[15], 30., 369.);
            line(c, "+ added line", a[2], 30., 391.);
            line(c, "− removed line", a[1], 170., 391.);
            line(c, "preview.ts", a[7], 30., 466.);
            tokens(
                c,
                &a,
                490.,
                &[
                    ("const ", 5),
                    ("theme", 15),
                    (" = ", 15),
                    ("\"aether\"", 2),
                    (";", 15),
                ],
            );
            line(c, "// wallpaper palette", a[8], 30., 514.);
        }
        PreviewTab::GlazeBar => {
            line(c, "상태 표시줄 실측 렌더링", c.ui.muted, 24., 112.);
            let bar_rect = Rect::new(c.p(20.), c.p(136.), c.width - c.p(40.), c.p(36.));
            c.fill(bar_rect, a[0]);
            c.frame(bar_rect, a[8]);

            // 워크스페이스 배지
            c.fill(
                Rect::new(c.p(26.), c.p(142.), c.p(28.), c.p(24.)),
                blend(a[0], a[4], 0.35),
            );
            c.text(
                "1",
                Rect::new(c.p(26.), c.p(146.), c.p(28.), c.p(18.)),
                a[4],
                12.,
                true,
                1,
            );
            c.text(
                "2",
                Rect::new(c.p(60.), c.p(146.), c.p(20.), c.p(18.)),
                a[7],
                12.,
                false,
                1,
            );
            c.text(
                "3",
                Rect::new(c.p(86.), c.p(146.), c.p(20.), c.p(18.)),
                a[7],
                12.,
                false,
                1,
            );

            // 중앙 타이틀
            c.text(
                "GlazeBar · Active Window",
                Rect::new(c.p(110.), c.p(146.), c.width - c.p(240.), c.p(18.)),
                a[15],
                12.,
                false,
                1,
            );

            // 우측 시스템 정보
            line(c, "CPU 12%", a[3], 240., 146.);
            line(c, "98% (충전 중)", a[2], 295., 146.);

            // 상세 컴포넌트 프리뷰
            line(c, "위젯 및 컬러 매핑", c.ui.muted, 24., 190.);
            for (i, (tag, col_idx, desc)) in [
                ("Primary Accent", 4, "활성 워크스페이스 / 슬라이더"),
                ("Success Status", 2, "배터리 충전 완료 / 양호"),
                ("Warning Status", 3, "배터리 부족 / CPU 경고"),
                ("Foreground Text", 15, "본문 텍스트 / 시계"),
                ("Subtle Muted", 8, "경계선 / 비활성 아이콘"),
            ]
            .iter()
            .enumerate()
            {
                let item_y = 215. + (i as f64 * 32.);
                c.fill(
                    Rect::new(c.p(24.), c.p(item_y), c.p(16.), c.p(16.)),
                    a[*col_idx],
                );
                c.text(
                    tag,
                    Rect::new(c.p(48.), c.p(item_y), c.p(120.), c.p(18.)),
                    a[15],
                    11.,
                    true,
                    0,
                );
                c.text(
                    desc,
                    Rect::new(c.p(175.), c.p(item_y), c.width - c.p(190.), c.p(18.)),
                    c.ui.muted,
                    11.,
                    false,
                    0,
                );
            }
        }
        PreviewTab::Terminal => {
            line(
                c,
                "가상 터미널 콘솔 (ANSI 16 Colors)",
                c.ui.muted,
                24.,
                112.,
            );
            let term_rect = Rect::new(c.p(20.), c.p(136.), c.width - c.p(40.), c.p(380.));
            c.fill(term_rect, a[0]);
            c.frame(term_rect, a[8]);

            line(c, "PowerShell 7.4.5 (x64)", a[8], 30., 148.);
            line(
                c,
                "Windows Terminal Profile: GlazeBar Theme",
                a[7],
                30.,
                168.,
            );
            tokens(
                c,
                &a,
                200.,
                &[("PS C:\\Users\\Dev> ", 6), ("git status", 15)],
            );
            line(
                c,
                "On branch feature/theme-ux-improvements",
                a[15],
                30.,
                225.,
            );
            line(
                c,
                "Your branch is up to date with 'origin/main'.",
                a[8],
                30.,
                245.,
            );
            line(c, "Changes not staged for commit:", a[3], 30., 275.);
            line(
                c,
                "  (use \"git add <file>...\" to update what will be committed)",
                a[8],
                30.,
                295.,
            );
            line(
                c,
                "	modified:   native-rust/src/theme_preview_ui.rs",
                a[1],
                30.,
                320.,
            );
            line(
                c,
                "	modified:   native-rust/src/wallpaper_ui.rs",
                a[2],
                30.,
                340.,
            );
            tokens(
                c,
                &a,
                375.,
                &[("PS C:\\Users\\Dev> ", 6), ("cargo check", 15)],
            );
            line(c, "   Compiling glazebar-native v0.0.11", a[2], 30., 400.);
            line(
                c,
                "    Finished dev [unoptimized + debuginfo] target(s)",
                a[2],
                30.,
                420.,
            );
            tokens(c, &a, 455., &[("PS C:\\Users\\Dev> ", 6), ("_", 15)]);
        }
        PreviewTab::VsCode => {
            line(
                c,
                "에디터 구문 강조 (Syntax Highlighting)",
                c.ui.muted,
                24.,
                112.,
            );
            let editor_rect = Rect::new(c.p(20.), c.p(136.), c.width - c.p(40.), c.p(380.));
            c.fill(editor_rect, a[0]);
            c.frame(editor_rect, a[8]);

            // 미니 탭 바
            c.fill(
                Rect::new(c.p(20.), c.p(136.), c.width - c.p(40.), c.p(28.)),
                blend(a[0], 0x000000, 0.2),
            );
            c.fill(Rect::new(c.p(24.), c.p(136.), c.p(120.), c.p(28.)), a[0]);
            c.text(
                "preview.rs  ×",
                Rect::new(c.p(34.), c.p(142.), c.p(100.), c.p(18.)),
                a[15],
                11.,
                false,
                0,
            );

            // 사이드 넘버 & 코드 라인들
            let code_lines: [&[(&str, usize)]; 9] = [
                &[("1 ", 8), ("use", 5), (" std::sync::Arc;", 15)],
                &[
                    ("2 ", 8),
                    ("pub struct", 5),
                    (" ThemePreview", 4),
                    (" {", 15),
                ],
                &[
                    ("3 ", 8),
                    ("    pub light", 15),
                    (":", 15),
                    (" bool", 6),
                    (",", 15),
                ],
                &[
                    ("4 ", 8),
                    ("    pub palette", 15),
                    (":", 15),
                    (" Option", 4),
                    ("<Palette>,", 15),
                ],
                &[("5 ", 8), ("}", 15)],
                &[("6 ", 8), ("impl", 5), (" ThemePreview {", 15)],
                &[
                    ("7 ", 8),
                    ("    pub fn", 5),
                    (" is_ready", 4),
                    ("(&", 15),
                    ("self", 3),
                    (") ->", 15),
                    (" bool", 6),
                    (" {", 15),
                ],
                &[
                    ("8 ", 8),
                    ("        self", 3),
                    (".palette.", 15),
                    ("is_some", 4),
                    ("()", 15),
                ],
                &[("9 ", 8), ("    }", 15)],
            ];

            for (idx, line_tokens) in code_lines.iter().enumerate() {
                tokens(c, &a, 180. + (idx as f64 * 24.), line_tokens);
            }
            tokens(c, &a, 410., &[("// OKLCH gamut & contrast checked", 8)]);
            tokens(
                c,
                &a,
                434.,
                &[
                    ("const", 5),
                    (" ACCENT_COLOR", 12),
                    (": &", 15),
                    ("str", 6),
                    (" = ", 15),
                    ("\"#3b82f6\"", 2),
                    (";", 15),
                ],
            );
        }
    }
}
