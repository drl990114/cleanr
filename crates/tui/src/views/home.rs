use super::*;

pub(crate) fn render_home(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    let mut content = bounded_content_rect(area, 82, 14);
    if area.height > content.height {
        content.y = content
            .y
            .saturating_add(area.height.saturating_sub(content.height) / 2);
    }

    let candidate_count = app.plan.as_ref().map_or_else(
        || {
            app.entries
                .iter()
                .filter(|entry| !entry.rule_hits.is_empty())
                .count()
        },
        |plan| plan.summary.candidate_count,
    );
    let (selected_count, selected_size) = app.plan.as_ref().map_or((0, 0), |plan| {
        (
            plan.summary.selected_count,
            plan.summary.selected_size_bytes,
        )
    });

    let lines = if app.scan_summary.entries_seen == 0 {
        vec![
            home_title(app.i18n.t("home_welcome"), app.theme),
            Line::from(Span::styled(
                app.i18n.t("home_subtitle"),
                Style::default().fg(app.theme.fg_dim),
            )),
            Line::from(""),
            home_action_line(app.theme, "s", app.i18n.t("home_action_scan"), true),
            home_action_line(app.theme, "u", app.i18n.t("home_action_usage"), false),
            home_action_line(app.theme, "/", app.i18n.t("home_action_more"), false),
            Line::from(""),
            home_safety_line(app),
        ]
    } else if candidate_count == 0 {
        vec![
            home_title(app.i18n.t("home_result_title"), app.theme),
            Line::from(Span::styled(
                app.i18n.t("home_result_empty"),
                Style::default().fg(app.theme.ok),
            )),
            Line::from(Span::styled(
                app.i18n.format(
                    "home_result_scanned",
                    &[("size", format_bytes(app.scan_summary.total_size_bytes))],
                ),
                Style::default().fg(app.theme.fg_dim),
            )),
            Line::from(""),
            home_action_line(app.theme, "s", app.i18n.t("home_action_rescan"), true),
            home_action_line(app.theme, "u", app.i18n.t("home_action_usage"), false),
            home_action_line(app.theme, "/", app.i18n.t("home_action_more"), false),
        ]
    } else {
        vec![
            home_title(app.i18n.t("home_result_title"), app.theme),
            Line::from(vec![
                Span::styled(
                    app.i18n.t("home_result_reclaimable"),
                    Style::default().fg(app.theme.fg_dim),
                ),
                Span::styled(
                    format_bytes(
                        app.plan
                            .as_ref()
                            .map_or(0, |plan| plan.summary.total_candidate_size_bytes),
                    ),
                    Style::default()
                        .fg(app.theme.cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    app.i18n.format(
                        "home_result_candidates",
                        &[("count", candidate_count.to_string())],
                    ),
                    Style::default().fg(app.theme.fg_dim),
                ),
            ]),
            Line::from(Span::styled(
                app.i18n.format(
                    "home_result_selected",
                    &[
                        ("count", selected_count.to_string()),
                        ("size", format_bytes(selected_size)),
                    ],
                ),
                Style::default().fg(app.theme.ok),
            )),
            Line::from(""),
            home_action_line(app.theme, "r", app.i18n.t("home_action_review"), true),
            home_action_line(app.theme, "u", app.i18n.t("home_action_usage"), false),
            home_action_line(app.theme, "s", app.i18n.t("home_action_rescan"), false),
            Line::from(""),
            home_safety_line(app),
        ]
    };

    frame.render_widget(
        Paragraph::new(lines).wrap(Wrap { trim: true }).block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(app.theme.border))
                .padding(Padding::new(2, 2, 1, 1)),
        ),
        content,
    );
}

pub(crate) fn home_title(title: String, theme: Theme) -> Line<'static> {
    Line::from(Span::styled(
        title,
        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD),
    ))
}

pub(crate) fn home_action_line(
    theme: Theme,
    key: &'static str,
    description: String,
    primary: bool,
) -> Line<'static> {
    let key_style = if primary {
        Style::default()
            .bg(theme.accent)
            .fg(theme.bg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .bg(theme.surface_alt)
            .fg(theme.fg)
            .add_modifier(Modifier::BOLD)
    };
    let description_style = if primary {
        Style::default().fg(theme.fg).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg_dim)
    };

    Line::from(vec![
        Span::styled(format!(" {key:^3} "), key_style),
        Span::raw("  "),
        Span::styled(description, description_style),
    ])
}

pub(crate) fn home_safety_line(app: &Workbench) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            "✓ ",
            Style::default()
                .fg(app.theme.ok)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.i18n.t("home_safety_note"),
            Style::default().fg(app.theme.fg_dim),
        ),
    ])
}
