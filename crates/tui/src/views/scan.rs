use super::*;

pub(crate) fn render_scan_workspace(frame: &mut Frame<'_>, area: Rect, app: &mut Workbench) {
    if app.is_scan_running() {
        render_scan_progress(frame, area, app, app.i18n.t("label_scan_tree"));
        return;
    }

    let wide = area.width >= 88;
    let workspace = bounded_content_rect(area, 164, area.height);
    let columns = responsive_workspace(workspace, 62);

    let right = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
        .split(columns[1]);

    render_candidates(frame, columns[0], app, wide);
    render_preview(frame, right[0], app);
    render_insight(frame, right[1], app);
    app.viewport_height = columns[0].height.saturating_sub(1).max(1);
}

pub(crate) fn render_scan_progress(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &Workbench,
    title: String,
) {
    let mut panel_area = bounded_content_rect(area, 96, 9);
    if area.height > panel_area.height {
        panel_area.y = panel_area.y.saturating_add(1);
    }
    let panel = Block::default()
        .borders(Borders::TOP | Borders::BOTTOM)
        .border_style(Style::default().fg(app.theme.border))
        .padding(Padding::horizontal(2))
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        );
    let inner = panel.inner(panel_area);
    frame.render_widget(panel, panel_area);

    let progress = app.scan_progress.as_ref();
    let phase = progress.map_or(ScanPhase::Discovering, |value| value.phase);
    let spinner = spinner_frame(app.animation_tick);
    let phase_line = Line::from(vec![
        Span::styled(
            format!("{spinner} "),
            Style::default()
                .fg(app.theme.warn)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            app.scan_phase_label(phase),
            Style::default()
                .fg(app.theme.fg)
                .add_modifier(Modifier::BOLD),
        ),
    ]);

    let (ratio, gauge_label) = progress.map_or((0.0, String::new()), |value| match value.phase {
        ScanPhase::Discovering => (
            ((app.animation_tick % 20) as f64 / 20.0).clamp(0.05, 0.95),
            app.i18n.format(
                "scan_progress_discovered",
                &[("total", value.entries_total.to_string())],
            ),
        ),
        ScanPhase::Scanning => {
            let ratio = if value.entries_total == 0 {
                ((app.animation_tick % 20) as f64 / 20.0).clamp(0.05, 0.95)
            } else {
                value.entries_scanned as f64 / value.entries_total as f64
            };
            (
                ratio.clamp(0.0, 1.0),
                if value.entries_total == 0 {
                    app.i18n.format(
                        "scan_progress_unbounded",
                        &[("scanned", value.entries_scanned.to_string())],
                    )
                } else {
                    app.i18n.format(
                        "scan_progress_count",
                        &[
                            ("scanned", value.entries_scanned.to_string()),
                            ("total", value.entries_total.to_string()),
                        ],
                    )
                },
            )
        }
        ScanPhase::Aggregating => (1.0, app.i18n.t("scan_progress_aggregating")),
    });

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);
    frame.render_widget(
        Paragraph::new(phase_line).alignment(ratatui::layout::Alignment::Center),
        rows[0],
    );
    frame.render_widget(
        Gauge::default()
            .ratio(ratio)
            .label(gauge_label)
            .gauge_style(
                Style::default()
                    .fg(app.theme.accent)
                    .bg(app.theme.surface_alt),
            ),
        rows[1],
    );

    let stats = progress.map_or_else(
        || app.i18n.t("scan_preparing"),
        |value| {
            app.i18n.format(
                "scan_progress_stats",
                &[
                    ("size", format_bytes(value.bytes_scanned)),
                    ("errors", value.errors.to_string()),
                ],
            )
        },
    );
    frame.render_widget(
        Paragraph::new(stats)
            .style(Style::default().fg(app.theme.fg_dim))
            .alignment(ratatui::layout::Alignment::Center),
        rows[2],
    );

    let current_path = progress
        .and_then(|value| value.current_path.as_ref())
        .map_or_else(
            || {
                if phase == ScanPhase::Aggregating {
                    app.i18n.t("scan_phase_aggregating")
                } else {
                    app.i18n.t("scan_preparing")
                }
            },
            |path| path.display().to_string(),
        );
    frame.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled(
                format!("{}  ", app.i18n.t("scan_current_path")),
                Style::default().fg(app.theme.fg_dim),
            ),
            Span::styled(current_path, Style::default().fg(app.theme.fg)),
        ])])
        .alignment(ratatui::layout::Alignment::Center)
        .wrap(Wrap { trim: true }),
        rows[3],
    );
    frame.render_widget(
        Paragraph::new(app.i18n.t("scan_cancel_hint"))
            .style(Style::default().fg(app.theme.fg_dim))
            .alignment(ratatui::layout::Alignment::Center),
        rows[4],
    );
}

pub(crate) fn render_candidates(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut Workbench,
    wide: bool,
) {
    let items: Vec<ListItem> = if let Some(plan) = &app.plan {
        plan.items
            .iter()
            .map(|item| {
                let check = if item.selected {
                    Span::styled("[✓]", Style::default().fg(app.theme.ok))
                } else {
                    Span::styled("[ ]", Style::default().fg(app.theme.fg_dim))
                };
                let size = Span::styled(
                    format!("{:>9} ", format_bytes(item.size_bytes)),
                    Style::default().fg(app.theme.cyan),
                );
                let icon =
                    Span::styled(kind_icon(item.kind), Style::default().fg(app.theme.accent));
                let path = Span::raw(compact_path(&item.path, &app.roots));
                let label = Span::styled(
                    format!("  · {}", item.category),
                    Style::default().fg(app.theme.fg_dim),
                );
                let conf = Span::styled(
                    format!(" {:?}", item.confidence).to_lowercase(),
                    Style::default().fg(confidence_color(item.confidence, app.theme)),
                );
                ListItem::new(Line::from(vec![
                    check,
                    Span::raw(" "),
                    size,
                    icon,
                    path,
                    label,
                    conf,
                ]))
            })
            .collect()
    } else {
        app.entries
            .iter()
            .filter(|entry| !entry.rule_hits.is_empty())
            .map(|entry| {
                let hit = &entry.rule_hits[0];
                let size = Span::styled(
                    format!("{:>9} ", format_bytes(entry.size_bytes)),
                    Style::default().fg(app.theme.cyan),
                );
                let icon =
                    Span::styled(kind_icon(entry.kind), Style::default().fg(app.theme.accent));
                let path = Span::raw(compact_path(&entry.path, &app.roots));
                let label = Span::styled(
                    format!("  · {}", hit.label),
                    Style::default().fg(app.theme.fg_dim),
                );
                let conf = Span::styled(
                    format!(" {:?}", hit.confidence).to_lowercase(),
                    Style::default().fg(confidence_color(hit.confidence, app.theme)),
                );
                ListItem::new(Line::from(vec![
                    Span::raw("  "),
                    size,
                    icon,
                    path,
                    label,
                    conf,
                ]))
            })
            .collect()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .borders(if wide {
                    Borders::TOP | Borders::RIGHT
                } else {
                    Borders::TOP
                })
                .border_style(Style::default().fg(app.theme.border))
                .title(format!(" {} ", app.i18n.t("label_scan_tree")))
                .title_style(
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.highlight_bg)
                .fg(app.theme.highlight_fg),
        )
        .highlight_symbol("› ");

    frame.render_stateful_widget(list, area, &mut app.list_state);
}

pub(crate) fn render_preview(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    let mut lines: Vec<Line> = Vec::new();

    if let Some(plan) = &app.plan {
        let selected_size = format_bytes(plan.summary.selected_size_bytes);
        lines.push(Line::from(vec![Span::styled(
            app.i18n.format(
                "plan_candidates",
                &[("count", plan.summary.candidate_count.to_string())],
            ),
            Style::default().fg(app.theme.fg),
        )]));
        lines.push(Line::from(vec![Span::styled(
            app.i18n.format(
                "plan_selected",
                &[("count", plan.summary.selected_count.to_string())],
            ),
            Style::default().fg(app.theme.ok),
        )]));
        lines.push(Line::from(vec![Span::styled(
            app.i18n
                .format("plan_selected_size", &[("size", selected_size)]),
            Style::default().fg(app.theme.cyan),
        )]));
        lines.push(Line::from(""));

        if let Some(idx) = app.list_state.selected()
            && let Some(item) = plan.items.get(idx)
        {
            lines.push(Line::from(vec![Span::styled(
                "Path",
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(item.path.display().to_string()));
            lines.push(Line::from(vec![
                Span::styled(
                    "Size",
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(": {}", format_bytes(item.size_bytes))),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    "Rule",
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(": {}", item.rule_id)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    "Reason",
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(": {}", item.reason)),
            ]));
            lines.push(Line::from(vec![
                Span::styled(
                    "Risk",
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(format!(": {}", item.risk_note)),
            ]));
            lines.push(Line::from(""));
        }

        lines.push(Line::from(vec![Span::styled(
            app.i18n.t("plan_export_hint"),
            Style::default().fg(app.theme.fg_dim),
        )]));
        lines.push(Line::from(vec![Span::styled(
            app.i18n.t("plan_clean_hint"),
            Style::default().fg(app.theme.fg_dim),
        )]));
    } else if app.is_scan_running() {
        lines.push(Line::from(app.i18n.t("plan_scanning")));
        lines.push(Line::from(app.i18n.t("plan_keep_typing")));
    } else {
        lines.push(Line::from(app.i18n.t("plan_empty")));
        lines.push(Line::from(app.i18n.t("plan_empty_hint")));
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(app.theme.border))
            .padding(Padding::horizontal(1))
            .title(format!(" {} ", app.i18n.t("label_preview")))
            .title_style(
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(paragraph, area);
}

pub(crate) fn render_insight(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    let mut lines: Vec<Line> = Vec::new();

    let current_path = app.plan.as_ref().and_then(|plan| {
        app.list_state
            .selected()
            .and_then(|idx| plan.items.get(idx).map(|item| item.path.clone()))
    });
    let target_matches = app
        .insight
        .target
        .as_ref()
        .zip(current_path.as_ref())
        .is_some_and(|(a, b)| a == b);

    let state: InsightState =
        if target_matches || matches!(app.insight.state, InsightState::Loading) {
            app.insight.state.clone()
        } else {
            InsightState::Empty
        };

    match state {
        InsightState::Empty => {
            lines.push(Line::from(vec![Span::styled(
                app.i18n.t("insight_empty"),
                Style::default().fg(app.theme.fg_dim),
            )]));
        }
        InsightState::Loading => {
            let spinner = spinner_frame(app.animation_tick);
            lines.push(Line::from(vec![
                Span::styled(spinner, Style::default().fg(app.theme.accent)),
                Span::raw(" "),
                Span::styled(
                    app.i18n.t("insight_loading"),
                    Style::default().fg(app.theme.fg_dim),
                ),
            ]));
        }
        InsightState::Ready(insight) => {
            lines.push(insight_line(
                app.i18n.t("insight_type"),
                insight.item_type.clone(),
                app.theme,
            ));
            lines.push(insight_line(
                app.i18n.t("insight_source"),
                insight.source.clone(),
                app.theme,
            ));
            lines.push(Line::from(""));
            lines.push(Line::from(vec![Span::styled(
                app.i18n.t("insight_meaning"),
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(Span::styled(
                insight.meaning.clone(),
                Style::default().fg(app.theme.fg),
            )));
            if !insight.referenced_by.is_empty() {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![Span::styled(
                    app.i18n.t("insight_referenced_by"),
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                )]));
                for reference in &insight.referenced_by {
                    lines.push(Line::from(Span::styled(
                        format!("  • {reference}"),
                        Style::default().fg(app.theme.fg),
                    )));
                }
            }
            lines.push(Line::from(""));
            lines.push(insight_line(
                app.i18n.t("insight_risk"),
                insight.risk.clone(),
                app.theme,
            ));
            lines.push(Line::from(vec![Span::styled(
                app.i18n.t("insight_advice"),
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]));
            lines.push(Line::from(Span::styled(
                insight.advice.clone(),
                Style::default().fg(app.theme.fg),
            )));
        }
        InsightState::Error(err) => {
            lines.push(Line::from(vec![Span::styled(
                app.i18n.t("insight_error"),
                Style::default().fg(app.theme.danger),
            )]));
            lines.push(Line::from(Span::styled(
                err.clone(),
                Style::default().fg(app.theme.fg_dim),
            )));
        }
    }

    let paragraph = Paragraph::new(lines).wrap(Wrap { trim: true }).block(
        Block::default()
            .borders(Borders::TOP)
            .border_style(Style::default().fg(app.theme.border))
            .padding(Padding::horizontal(1))
            .title(format!(" {} ", app.i18n.t("label_insight")))
            .title_style(
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );

    frame.render_widget(paragraph, area);
}

pub(crate) fn insight_line(label: String, value: String, theme: Theme) -> Line<'static> {
    let value_style = match value.to_lowercase().as_str() {
        "low" => Style::default().fg(theme.ok),
        "medium" => Style::default().fg(theme.warn),
        "high" => Style::default().fg(theme.danger),
        _ => Style::default().fg(theme.fg),
    };
    Line::from(vec![
        Span::styled(
            label,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(value, value_style),
    ])
}
