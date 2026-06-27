use cleanr_config::UiTheme;
use ratatui::style::Color;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub surface_alt: Color,
    pub fg: Color,
    pub fg_dim: Color,
    pub border: Color,
    pub accent: Color,
    pub ok: Color,
    pub warn: Color,
    pub danger: Color,
    pub cyan: Color,
    pub magenta: Color,
    pub highlight_bg: Color,
    pub highlight_fg: Color,
}

impl Theme {
    /// Neutral dark palette with a restrained blue accent.
    pub const fn dark() -> Self {
        Self {
            bg: Color::Rgb(18, 19, 23),
            surface: Color::Rgb(23, 25, 30),
            surface_alt: Color::Rgb(31, 34, 41),
            fg: Color::Rgb(224, 227, 234),
            fg_dim: Color::Rgb(132, 138, 151),
            border: Color::Rgb(48, 52, 61),
            accent: Color::Rgb(116, 158, 232),
            ok: Color::Rgb(102, 181, 137),
            warn: Color::Rgb(213, 164, 92),
            danger: Color::Rgb(210, 104, 116),
            cyan: Color::Rgb(102, 174, 188),
            magenta: Color::Rgb(167, 130, 191),
            highlight_bg: Color::Rgb(34, 38, 47),
            highlight_fg: Color::Rgb(239, 241, 246),
        }
    }

    /// Neutral light palette with soft surfaces and crisp text.
    pub const fn light() -> Self {
        Self {
            bg: Color::Rgb(250, 250, 251),
            surface: Color::Rgb(245, 246, 248),
            surface_alt: Color::Rgb(235, 238, 243),
            fg: Color::Rgb(32, 35, 41),
            fg_dim: Color::Rgb(112, 118, 130),
            border: Color::Rgb(214, 218, 225),
            accent: Color::Rgb(66, 101, 173),
            ok: Color::Rgb(48, 132, 86),
            warn: Color::Rgb(169, 108, 38),
            danger: Color::Rgb(184, 65, 76),
            cyan: Color::Rgb(43, 124, 138),
            magenta: Color::Rgb(124, 83, 157),
            highlight_bg: Color::Rgb(232, 237, 246),
            highlight_fg: Color::Rgb(25, 36, 57),
        }
    }
}

pub(crate) fn resolve_theme(theme: UiTheme) -> Theme {
    match theme {
        UiTheme::Dark => Theme::dark(),
        UiTheme::Light => Theme::light(),
        UiTheme::Auto => {
            if terminal_light::luma().is_ok_and(|luma| luma > 0.6) {
                Theme::light()
            } else {
                Theme::dark()
            }
        }
    }
}
