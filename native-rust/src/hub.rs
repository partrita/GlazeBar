use crate::{
    graphics::{Canvas, Rect},
    settings_ui::BarSettings,
    views::{self, Action, Hit},
    wallpaper_ui::Picker,
};
use std::path::Path;

/// Preferred window size shared by every hub tab. Both embedded screens are
/// laid out for this content area.
pub const WINDOW_SIZE: (f64, f64) = (1000., 660.);
/// Height of the tab strip above the embedded screen content.
pub const HEADER_HEIGHT: f64 = 52.;
const TAB_WIDTH: f64 = 132.;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Settings,
    Theme,
}

impl Tab {
    fn label(self) -> &'static str {
        match self {
            Self::Settings => "일반 설정",
            Self::Theme => "테마 · 배경화면",
        }
    }
}

/// One window that hosts both the general settings and the theme/wallpaper
/// screens. Keeping both states alive means switching tabs is instant and does
/// not reload wallpaper thumbnails or discard unsaved picker choices.
#[derive(Clone)]
pub struct Hub {
    pub tab: Tab,
    pub settings: BarSettings,
    pub picker: Option<Picker>,
    pub theme_error: Option<String>,
}

impl Hub {
    pub fn new(root: &Path, tab: Tab) -> Self {
        let settings = BarSettings::load(root);
        let (picker, theme_error) = match Picker::load(root) {
            Ok(picker) => (Some(picker), None),
            Err(error) => (None, Some(error.to_string())),
        };
        Self {
            tab,
            settings,
            picker,
            theme_error,
        }
    }

    pub fn picker_mut(&mut self) -> Option<&mut Picker> {
        self.picker.as_mut()
    }

    pub fn reload_theme(&mut self, root: &Path) {
        match Picker::load(root) {
            Ok(picker) => {
                self.picker = Some(picker);
                self.theme_error = None;
            }
            Err(error) => {
                self.picker = None;
                self.theme_error = Some(error.to_string());
            }
        }
    }

    pub fn switch(&mut self, tab: Tab) {
        self.tab = tab;
        self.settings.open_dropdown = None;
        if tab == Tab::Theme {
            self.sync_targets();
        }
        if let Some(picker) = &mut self.picker {
            picker.open_dropdown = None;
        }
    }

    /// The general settings tab owns the apply-target list; the theme tab only
    /// displays and applies it. Copy the latest choices before applying.
    fn sync_targets(&mut self) {
        let Some(picker) = &mut self.picker else {
            return;
        };
        let targets = self.settings.targets;
        picker.targets = [targets[0], targets[1], targets[2], targets[3]];
        picker.extra_targets.copy_from_slice(&targets[4..16]);
    }

    pub fn is_theme_tab(&self) -> bool {
        self.tab == Tab::Theme
    }
}

pub fn paint(c: &mut Canvas, hub: &Hub, hover: Option<(i32, i32)>) -> Vec<Hit> {
    let scale = c.scale;
    let s = |n: f64| (n * scale).round_ties_even() as i32;
    let width = c.width;
    let height = c.height;
    let header = s(HEADER_HEIGHT);
    let mut hits = Vec::new();

    c.fill(Rect::new(0, 0, width, header), c.ui.surface);

    // Tab strip.
    for (index, tab) in [Tab::Settings, Tab::Theme].into_iter().enumerate() {
        let r = Rect::new(
            s(20.) + index as i32 * (s(TAB_WIDTH) + s(6.)),
            s(10.),
            s(TAB_WIDTH),
            s(32.),
        );
        views::button(
            c,
            &mut hits,
            tab.label(),
            r,
            hub.tab == tab,
            hover.is_some_and(|(x, y)| r.contains(x, y)),
            true,
            Action::HubTab(tab),
        );
    }

    // Hint for keyboard users, kept subtle so it never competes with tabs.
    c.text(
        "Ctrl+Tab 전환",
        Rect::new(s(20.) + 2 * (s(TAB_WIDTH) + s(6.)), s(10.), s(110.), s(32.)),
        c.ui.muted,
        11.,
        false,
        0,
    );

    let close = Rect::new(width - s(92.), s(11.), s(72.), s(30.));
    views::button(
        c,
        &mut hits,
        "닫기",
        close,
        false,
        hover.is_some_and(|(x, y)| close.contains(x, y)),
        true,
        Action::Close,
    );
    c.fill(Rect::new(0, header - 1, width, 1), c.ui.border);

    // Draw the active screen inside the area below the tab strip.
    c.origin = (0, header);
    c.height = height - header;
    let content_hover = hover.map(|(x, y)| (x, y - header));
    let mut content_hits: Vec<Hit> = match hub.tab {
        Tab::Settings => crate::settings_ui::paint(c, &hub.settings, content_hover, false),
        Tab::Theme => match &hub.picker {
            Some(picker) => crate::wallpaper_ui::paint(c, picker, content_hover, false)
                .into_iter()
                .map(|hit| Hit {
                    rect: hit.rect,
                    action: Action::Picker(hit.action),
                })
                .collect(),
            None => {
                let message = hub
                    .theme_error
                    .as_deref()
                    .unwrap_or("배경화면 폴더를 불러올 수 없습니다");
                c.text(
                    message,
                    Rect::new(s(24.), s(90.), c.width - s(48.), s(30.)),
                    c.ui.muted,
                    12.5,
                    false,
                    1,
                );
                let retry = Rect::new((c.width - s(140.)) / 2, s(132.), s(140.), s(34.));
                let mut extra = Vec::new();
                views::button(
                    c,
                    &mut extra,
                    "다시 시도",
                    retry,
                    false,
                    content_hover.is_some_and(|(x, y)| retry.contains(x, y)),
                    true,
                    Action::HubRetryTheme,
                );
                extra
            }
        },
    };
    c.height = height;
    c.origin = (0, 0);

    for hit in &mut content_hits {
        hit.rect.y += header;
    }
    hits.extend(content_hits);
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hits_are_translated_below_the_tab_strip() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap();
        let output = tempfile::tempdir().unwrap();
        let hub = Hub {
            tab: Tab::Theme,
            settings: BarSettings::default(),
            picker: None,
            theme_error: Some("테스트 오류".into()),
        };
        let mut hits = Vec::new();
        crate::graphics::render_png(
            root,
            &output.path().join("hub.png"),
            WINDOW_SIZE.0 as i32,
            WINDOW_SIZE.1 as i32,
            1.,
            crate::graphics::Palette::load(root),
            |canvas| {
                hits = paint(canvas, &hub, None);
            },
        )
        .unwrap();
        let tab = hits
            .iter()
            .find(|hit| hit.action == Action::HubTab(Tab::Settings))
            .expect("settings tab hit");
        assert_eq!(tab.rect.y, 10, "tab strip stays in window coordinates");
        let retry = hits
            .iter()
            .find(|hit| hit.action == Action::HubRetryTheme)
            .expect("retry hit");
        assert_eq!(
            retry.rect.y,
            52 + 132,
            "content hits must be offset below the tab strip"
        );
    }
}
