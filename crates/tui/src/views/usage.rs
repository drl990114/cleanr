use super::*;

pub(crate) fn render_usage(frame: &mut Frame<'_>, area: Rect, app: &mut Workbench) {
    if app.is_scan_running() {
        render_scan_progress(frame, area, app, app.i18n.t("label_usage"));
        return;
    }

    let entries = usage_entries(app);
    let max_size = entries.iter().map(|e| e.size_bytes).max().unwrap_or(0);
    let bar_width = usage_bar_width(area.width);

    let items: Vec<ListItem<'static>> = if entries.is_empty() {
        vec![ListItem::new(Line::from(vec![Span::styled(
            app.i18n.t("status_no_scan_results"),
            Style::default().fg(app.theme.fg_dim),
        )]))]
    } else {
        entries
            .iter()
            .map(|entry| ListItem::new(usage_bar_line(entry, max_size, bar_width, app.theme)))
            .collect()
    };

    let details = app
        .list_state
        .selected()
        .and_then(|idx| entries.get(idx))
        .map_or_else(
            || {
                vec![
                    detail_line("Roots", join_paths(&app.roots), app.theme.fg_dim, app.theme),
                    detail_line(
                        "Total",
                        format_bytes(app.scan_summary.total_size_bytes),
                        app.theme.cyan,
                        app.theme,
                    ),
                    detail_line(
                        "Entries",
                        app.scan_summary.entries_seen.to_string(),
                        app.theme.fg,
                        app.theme,
                    ),
                    Line::from(vec![Span::styled(
                        app.i18n.t("usage_context_hint"),
                        Style::default().fg(app.theme.fg_dim),
                    )]),
                ]
            },
            |entry| {
                vec![
                    detail_line(
                        "Path",
                        entry.path.display().to_string(),
                        app.theme.fg_dim,
                        app.theme,
                    ),
                    detail_line(
                        "Size",
                        format_bytes(entry.size_bytes),
                        app.theme.cyan,
                        app.theme,
                    ),
                    detail_line(
                        "Kind",
                        kind_label(entry.kind).to_string(),
                        app.theme.fg,
                        app.theme,
                    ),
                    detail_line(
                        "Contained",
                        usage_descendant_count(&app.entries, entry).to_string(),
                        app.theme.ok,
                        app.theme,
                    ),
                    detail_line(
                        "Rule hits",
                        entry.rule_hits.len().to_string(),
                        app.theme.warn,
                        app.theme,
                    ),
                ]
            },
        );

    let title = app.i18n.t("label_usage");
    let detail_title = app.i18n.t("label_details");

    let candidates = app.plan.as_ref().map_or_else(
        || {
            app.entries
                .iter()
                .filter(|entry| !entry.rule_hits.is_empty())
                .count()
        },
        |plan| plan.summary.candidate_count,
    );
    let selected = app
        .plan
        .as_ref()
        .map_or(0, |plan| plan.summary.selected_count);
    let overview = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled(
            app.i18n.t("usage_overview"),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(app.theme.border)),
        metric_span(
            app.i18n.t("usage_metric_total"),
            format_bytes(app.scan_summary.total_size_bytes),
            app.theme.cyan,
        ),
        Span::styled("  ·  ", Style::default().fg(app.theme.border)),
        metric_span(
            app.i18n.t("usage_metric_entries"),
            app.scan_summary.entries_seen.to_string(),
            app.theme.fg,
        ),
        Span::styled("  ·  ", Style::default().fg(app.theme.border)),
        metric_span(
            app.i18n.t("usage_metric_candidates"),
            candidates.to_string(),
            app.theme.warn,
        ),
        Span::styled("  ·  ", Style::default().fg(app.theme.border)),
        metric_span(
            app.i18n.t("usage_metric_selected"),
            selected.to_string(),
            app.theme.ok,
        ),
    ]))
    .wrap(Wrap { trim: true })
    .style(Style::default().bg(app.theme.surface));
    let summary_area = bounded_content_rect(area, 164, 1);
    frame.render_widget(overview, summary_area);

    let content_area = Rect::new(
        area.x,
        summary_area.y.saturating_add(summary_area.height),
        area.width,
        area.bottom()
            .saturating_sub(summary_area.y.saturating_add(summary_area.height)),
    );
    app.viewport_height = render_context_workspace(
        frame,
        content_area,
        &mut app.list_state,
        app.theme,
        title,
        items,
        detail_title,
        details,
    );
}

pub(crate) fn usage_entries(app: &Workbench) -> Vec<&ScanEntry> {
    let mut result = Vec::new();
    for root in &app.roots {
        let root_path = root.as_path();
        let mut children: Vec<&ScanEntry> = app
            .entries
            .iter()
            .filter(|e| e.path.parent() == Some(root_path))
            .collect();
        children.sort_by_key(|e| std::cmp::Reverse(e.size_bytes));
        result.extend(children);
    }
    if result.is_empty() {
        let mut all: Vec<&ScanEntry> = app.entries.iter().collect();
        all.sort_by_key(|e| std::cmp::Reverse(e.size_bytes));
        result.extend(all.into_iter().take(100));
    }
    result
}

pub(crate) fn usage_descendant_count(entries: &[ScanEntry], parent: &ScanEntry) -> usize {
    if parent.kind != EntryKind::Directory {
        return 0;
    }
    entries
        .iter()
        .filter(|entry| entry.path != parent.path && entry.path.starts_with(&parent.path))
        .count()
}

pub(crate) fn usage_bar_line(
    entry: &ScanEntry,
    max_size: u64,
    bar_width: usize,
    theme: Theme,
) -> Line<'static> {
    let size_str = format_bytes(entry.size_bytes);
    let filled = if max_size == 0 {
        0
    } else {
        ((entry.size_bytes as f64 / max_size as f64) * bar_width as f64).round() as usize
    };
    let empty = bar_width.saturating_sub(filled);
    let bar = format!("{}{}", "█".repeat(filled), "░".repeat(empty));

    let name = entry
        .path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| entry.path.display().to_string());
    let icon = kind_icon(entry.kind);
    let suffix = if entry.kind == EntryKind::Directory {
        "/"
    } else {
        ""
    };

    Line::from(vec![
        Span::styled(format!("{size_str:>10}"), Style::default().fg(theme.cyan)),
        Span::raw(" ["),
        Span::styled(bar, Style::default().fg(theme.accent)),
        Span::raw("] "),
        Span::styled(
            format!("{icon}{name}{suffix}"),
            Style::default().fg(theme.fg),
        ),
    ])
}

pub(crate) fn usage_bar_width(area_width: u16) -> usize {
    match area_width {
        0..=54 => 4,
        55..=84 => 8,
        85..=124 => 12,
        _ => 18,
    }
}
