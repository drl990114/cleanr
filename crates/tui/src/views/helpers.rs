use super::*;

// -------------------------------------------------------------------------
// Helpers
// -------------------------------------------------------------------------
pub(crate) fn detail_line(
    label: &'static str,
    value: String,
    value_color: Color,
    theme: Theme,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{label}: "), Style::default().fg(theme.fg_dim)),
        Span::styled(value, Style::default().fg(value_color)),
    ])
}

pub(crate) fn view_title(app: &Workbench) -> String {
    let key = match app.view {
        View::Home => "label_home",
        View::Scan => "label_scan_tree",
        View::Languages => "label_languages",
        View::Rules => "label_rules",
        View::Plugins => "label_plugins",
        View::Tasks => "label_tasks",
        View::Usage => "label_usage",
        View::Restore => "label_restore",
    };
    app.i18n.t(key)
}

pub(crate) fn key_hint(key: &'static str, label: String, theme: Theme) -> [Span<'static>; 2] {
    [
        Span::styled(
            key,
            Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {label}   "), Style::default().fg(theme.fg_dim)),
    ]
}

pub(crate) fn compact_path(path: &std::path::Path, roots: &[PathBuf]) -> String {
    roots
        .iter()
        .find_map(|root| path.strip_prefix(root).ok())
        .filter(|relative| !relative.as_os_str().is_empty())
        .map_or_else(
            || path.display().to_string(),
            |relative| relative.display().to_string(),
        )
}

pub(crate) fn kind_label(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Directory => "directory",
        EntryKind::File => "file",
        EntryKind::Symlink => "symlink",
        EntryKind::Other => "other",
    }
}

pub(crate) fn language_source_label(source: &LanguagePackSource) -> &'static str {
    match source {
        LanguagePackSource::Builtin => "builtin",
        LanguagePackSource::UserFile(_) => "user",
        LanguagePackSource::Plugin { .. } => "plugin",
    }
}

pub(crate) fn join_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return "-".to_string();
    }
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn kind_icon(kind: EntryKind) -> &'static str {
    match kind {
        EntryKind::Directory => "▣ ",
        EntryKind::Symlink => "↗ ",
        EntryKind::File => "· ",
        EntryKind::Other => "? ",
    }
}

pub(crate) fn confidence_color(confidence: cleanr_core::Confidence, theme: Theme) -> Color {
    match confidence {
        cleanr_core::Confidence::High => theme.ok,
        cleanr_core::Confidence::Medium => theme.warn,
        cleanr_core::Confidence::Low => theme.danger,
    }
}

pub(crate) fn metric_span(label: String, value: String, value_color: Color) -> Span<'static> {
    Span::styled(
        format!("{label} {value}"),
        Style::default()
            .fg(value_color)
            .add_modifier(Modifier::BOLD),
    )
}

pub(crate) fn responsive_workspace(area: Rect, list_percent: u16) -> [Rect; 2] {
    let chunks = if area.width >= 88 {
        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(list_percent),
                Constraint::Percentage(100u16.saturating_sub(list_percent)),
            ])
            .split(area)
    } else {
        Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(62), Constraint::Percentage(38)])
            .split(area)
    };
    [chunks[0], chunks[1]]
}

pub(crate) fn bounded_content_rect(area: Rect, max_width: u16, desired_height: u16) -> Rect {
    let width = area.width.min(max_width);
    let height = area.height.min(desired_height);
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y,
        width,
        height,
    )
}

pub(crate) fn ime_guard_position(area: Rect) -> Position {
    Position::new(
        area.right().saturating_sub(2),
        area.bottom().saturating_sub(2),
    )
}

pub(crate) fn command_cursor_position(area: Rect, input: &str) -> Option<Position> {
    if area.is_empty() {
        return None;
    }
    let prefix_width = 3usize;
    let input_width = input
        .chars()
        .next()
        .map_or(0, char::len_utf8)
        .min(input.len());
    let rest_width = Line::from(input[input_width..].to_string()).width();
    let offset = u16::try_from(prefix_width.saturating_add(rest_width)).unwrap_or(u16::MAX);
    Some(Position::new(
        area.x
            .saturating_add(offset)
            .min(area.right().saturating_sub(1)),
        area.y,
    ))
}

pub(crate) fn centered_bounded_rect(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    max_width: u16,
) -> Rect {
    let width = area.width.min(desired_width.max(24).min(max_width));
    let height = area.height.min(desired_height.max(3));
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height) / 2,
        width,
        height,
    )
}

pub(crate) fn bottom_bounded_rect(
    area: Rect,
    desired_width: u16,
    desired_height: u16,
    max_width: u16,
) -> Rect {
    let width = area.width.min(desired_width.max(24).min(max_width));
    let height = area.height.min(desired_height.max(3));
    Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.bottom().saturating_sub(height),
        width,
        height,
    )
}

pub(crate) fn spinner_frame(tick: u64) -> &'static str {
    const FRAMES: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];
    FRAMES[tick as usize % FRAMES.len()]
}

pub(crate) fn format_bytes(bytes: u64) -> String {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    let bytes_f = bytes as f64;
    if bytes_f >= GB {
        format!("{:.2} GiB", bytes_f / GB)
    } else if bytes_f >= MB {
        format!("{:.2} MiB", bytes_f / MB)
    } else if bytes_f >= KB {
        format!("{:.2} KiB", bytes_f / KB)
    } else {
        format!("{bytes} B")
    }
}
