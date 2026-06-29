use std::{io, path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use cleanr_config::Config;
use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event},
    execute,
    terminal::{
        Clear as TerminalClear, ClearType, EnterAlternateScreen, LeaveAlternateScreen,
        disable_raw_mode, enable_raw_mode,
    },
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::{
    app::{Mode, Workbench},
    effects::load_runtime,
    theme::resolve_theme,
    views::{ime_guard_position, render},
};

pub struct TuiOptions {
    pub roots: Vec<PathBuf>,
    pub config: Config,
    pub update_available: Option<UpdateNotice>,
}

pub struct UpdateNotice {
    pub version: String,
    pub release_url: String,
}

pub fn run(options: TuiOptions) -> Result<()> {
    let (registry, i18n) = load_runtime(&options.config)?;
    let theme = resolve_theme(options.config.ui.theme);
    let mut app = Workbench::new(options.roots, options.config, registry, i18n, theme)?;
    if let Some(update) = options.update_available {
        app.status = app.i18n.format(
            "status_update_available",
            &[("version", update.version), ("url", update.release_url)],
        );
    }

    enable_raw_mode().context("failed to enable raw mode")?;
    let _guard = TerminalGuard;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        TerminalClear(ClearType::All),
        MoveTo(0, 0),
        Hide
    )
    .context("failed to enter alternate screen")?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).context("failed to initialize terminal")?;
    terminal
        .clear()
        .context("failed to clear terminal before rendering")?;

    loop {
        app.poll_tasks();
        let area = terminal.draw(|frame| render(frame, &mut app))?.area;
        if matches!(app.mode, Mode::Normal) {
            terminal.set_cursor_position(ime_guard_position(area))?;
            terminal.hide_cursor()?;
        }

        if app.should_quit {
            break;
        }

        if event::poll(Duration::from_millis(100))? {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            app.handle_key(key);
        }
    }

    Ok(())
}

struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), Show, LeaveAlternateScreen);
    }
}
