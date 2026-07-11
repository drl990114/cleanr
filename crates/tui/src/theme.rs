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
    /// Portable dark palette that inherits the terminal background.
    pub const fn dark() -> Self {
        Self {
            bg: Color::Reset,
            surface: Color::Reset,
            surface_alt: Color::DarkGray,
            fg: Color::Reset,
            fg_dim: Color::DarkGray,
            border: Color::DarkGray,
            accent: Color::Cyan,
            ok: Color::Green,
            warn: Color::Yellow,
            danger: Color::Red,
            cyan: Color::Cyan,
            magenta: Color::Magenta,
            highlight_bg: Color::Reset,
            highlight_fg: Color::Cyan,
        }
    }

    /// Portable light palette that inherits the terminal background.
    pub const fn light() -> Self {
        Self {
            bg: Color::Reset,
            surface: Color::Reset,
            surface_alt: Color::Gray,
            fg: Color::Reset,
            fg_dim: Color::DarkGray,
            border: Color::Gray,
            accent: Color::Blue,
            ok: Color::Green,
            warn: Color::Yellow,
            danger: Color::Red,
            cyan: Color::Cyan,
            magenta: Color::Magenta,
            highlight_bg: Color::Reset,
            highlight_fg: Color::Blue,
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
