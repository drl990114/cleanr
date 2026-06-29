use super::*;

pub(crate) fn render_scan_workspace(frame: &mut Frame<'_>, area: Rect, app: &mut Workbench) {
    if app.is_scan_running() {
        render_scan_progress(frame, area, app, app.i18n.t("label_scan_tree"));
        return;
    }

    let wide = area.width >= 88;
    let workspace = fluid_content_rect(area, 220, area.height);
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
    let mut panel_area = fluid_content_rect(area, 220, area.height);
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
    let progress_label = scan_progress_label(progress, app);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);
    let heading = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(52), Constraint::Percentage(48)])
        .split(rows[0]);
    let phase_line = Line::from(vec![
        Span::styled(
            format!("{} ", spinner_frame(app.animation_tick)),
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
    frame.render_widget(
        Paragraph::new(phase_line).alignment(ratatui::layout::Alignment::Left),
        heading[0],
    );
    frame.render_widget(
        Paragraph::new(progress_label)
            .style(Style::default().fg(app.theme.fg_dim))
            .alignment(ratatui::layout::Alignment::Right),
        heading[1],
    );
    frame.render_widget(Paragraph::new(scan_stage_line(phase, app)), rows[1]);
    frame.render_widget(
        Paragraph::new(activity_bar_line(
            rows[2].width as usize,
            app.animation_tick,
            app.theme,
        )),
        rows[2],
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
            .alignment(ratatui::layout::Alignment::Left),
        rows[3],
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
    let current_path_label = format!("{}  ", app.i18n.t("scan_current_path"));
    let current_path_width = rows[4]
        .width
        .saturating_sub(u16::try_from(display_width(&current_path_label)).unwrap_or(u16::MAX))
        as usize;
    let current_path = truncate_text(&current_path, current_path_width);
    frame.render_widget(
        Paragraph::new(vec![Line::from(vec![
            Span::styled(current_path_label, Style::default().fg(app.theme.fg_dim)),
            Span::styled(current_path, Style::default().fg(app.theme.fg)),
        ])])
        .alignment(ratatui::layout::Alignment::Left)
        .wrap(Wrap { trim: true }),
        rows[4],
    );
    frame.render_widget(
        Paragraph::new(app.i18n.t("scan_cancel_hint"))
            .style(Style::default().fg(app.theme.fg_dim))
            .alignment(ratatui::layout::Alignment::Right),
        rows[5],
    );
}

fn scan_progress_label(progress: Option<&cleanr_fs::ScanProgress>, app: &Workbench) -> String {
    progress.map_or_else(
        || app.i18n.t("scan_preparing"),
        |value| match value.phase {
            ScanPhase::Discovering => app.i18n.format(
                "scan_progress_discovered",
                &[("total", value.entries_total.to_string())],
            ),
            ScanPhase::Scanning => {
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
                }
            }
            ScanPhase::Aggregating => app.i18n.t("scan_progress_aggregating"),
        },
    )
}

fn scan_stage_line(phase: ScanPhase, app: &Workbench) -> Line<'static> {
    let stages = [ScanPhase::Scanning, ScanPhase::Aggregating];
    let current = stage_index(phase);
    let mut spans = Vec::new();

    for (index, stage) in stages.into_iter().enumerate() {
        if index > 0 {
            spans.push(Span::styled(" ── ", Style::default().fg(app.theme.border)));
        }
        let style = if index < current {
            Style::default().fg(app.theme.ok)
        } else if index == current {
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(app.theme.fg_dim)
        };
        let marker = if index < current {
            "✓"
        } else if index == current {
            "●"
        } else {
            "○"
        };
        spans.push(Span::styled(
            format!("{marker} {}", app.scan_phase_label(stage)),
            style,
        ));
    }

    Line::from(spans)
}

fn stage_index(phase: ScanPhase) -> usize {
    match phase {
        ScanPhase::Discovering | ScanPhase::Scanning => 0,
        ScanPhase::Aggregating => 1,
    }
}

fn activity_bar_line(width: usize, animation_tick: u64, theme: Theme) -> Line<'static> {
    if width == 0 {
        return Line::from("");
    }

    let pulse = (width / 5).clamp(8, 24).min(width);
    let cycle = width.saturating_add(pulse).max(1);
    let head = ((animation_tick as usize)
        .wrapping_mul(3)
        .saturating_add(pulse))
        % cycle;
    let start = head.saturating_sub(pulse).min(width);
    let end = head.min(width);

    Line::from(vec![
        Span::styled("─".repeat(start), Style::default().fg(theme.surface_alt)),
        Span::styled(
            "━".repeat(end.saturating_sub(start)),
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "─".repeat(width.saturating_sub(end)),
            Style::default().fg(theme.surface_alt),
        ),
    ])
}

#[cfg(test)]
pub(crate) fn scan_loading_bar_sample(width: usize, animation_tick: u64, theme: Theme) -> String {
    activity_bar_line(width, animation_tick, theme)
        .spans
        .into_iter()
        .map(|span| span.content.into_owned())
        .collect()
}

pub(crate) fn render_candidates(
    frame: &mut Frame<'_>,
    area: Rect,
    app: &mut Workbench,
    wide: bool,
) {
    let content_width = candidate_content_width(area, wide);
    let items: Vec<ListItem> = if let Some(plan) = &app.plan {
        plan.items
            .iter()
            .map(|item| {
                ListItem::new(plan_candidate_line(
                    item,
                    &app.roots,
                    app.theme,
                    content_width,
                ))
            })
            .collect()
    } else {
        app.entries
            .iter()
            .filter(|entry| !entry.rule_hits.is_empty())
            .map(|entry| {
                ListItem::new(scan_candidate_line(
                    entry,
                    &app.roots,
                    app.theme,
                    content_width,
                ))
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

fn candidate_content_width(area: Rect, wide: bool) -> usize {
    let right_border = if wide { 1 } else { 0 };
    area.width
        .saturating_sub(right_border)
        .saturating_sub(2)
        .into()
}

fn plan_candidate_line(
    item: &CleanupItem,
    roots: &[PathBuf],
    theme: Theme,
    content_width: usize,
) -> Line<'static> {
    let check_text = if item.selected { "[✓]" } else { "[ ]" };
    let check = if item.selected {
        Span::styled(check_text, Style::default().fg(theme.ok))
    } else {
        Span::styled(check_text, Style::default().fg(theme.fg_dim))
    };
    let size_text = size_cell(item.size_bytes);
    let icon_text = kind_icon(item.kind);
    let label_text = format!("  · {}", item.category);
    let confidence_text = format!(" {}", confidence_label(item.confidence));
    let fixed_width = display_width(check_text)
        + 1
        + display_width(&size_text)
        + display_width(icon_text)
        + display_width(&label_text)
        + display_width(&confidence_text);
    let path_width = content_width.saturating_sub(fixed_width);

    Line::from(vec![
        check,
        Span::raw(" "),
        Span::styled(size_text, Style::default().fg(theme.cyan)),
        Span::styled(icon_text, Style::default().fg(theme.accent)),
        Span::raw(compact_path_for_width(&item.path, roots, path_width)),
        Span::styled(label_text, Style::default().fg(theme.fg_dim)),
        Span::styled(
            confidence_text,
            Style::default().fg(confidence_color(item.confidence, theme)),
        ),
    ])
}

fn scan_candidate_line(
    entry: &ScanEntry,
    roots: &[PathBuf],
    theme: Theme,
    content_width: usize,
) -> Line<'static> {
    let hit = &entry.rule_hits[0];
    let size_text = size_cell(entry.size_bytes);
    let icon_text = kind_icon(entry.kind);
    let label_text = format!("  · {}", hit.label);
    let confidence_text = format!(" {}", confidence_label(hit.confidence));
    let fixed_width = 2
        + display_width(&size_text)
        + display_width(icon_text)
        + display_width(&label_text)
        + display_width(&confidence_text);
    let path_width = content_width.saturating_sub(fixed_width);

    Line::from(vec![
        Span::raw("  "),
        Span::styled(size_text, Style::default().fg(theme.cyan)),
        Span::styled(icon_text, Style::default().fg(theme.accent)),
        Span::raw(compact_path_for_width(&entry.path, roots, path_width)),
        Span::styled(label_text, Style::default().fg(theme.fg_dim)),
        Span::styled(
            confidence_text,
            Style::default().fg(confidence_color(hit.confidence, theme)),
        ),
    ])
}

fn size_cell(bytes: u64) -> String {
    format!("{:>10} ", format_bytes(bytes))
}

fn confidence_label(confidence: Confidence) -> &'static str {
    match confidence {
        Confidence::High => "high",
        Confidence::Medium => "medium",
        Confidence::Low => "low",
    }
}

pub(crate) fn render_preview(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    let mut lines: Vec<Line> = Vec::new();
    let inner_width = area.width.saturating_sub(2) as usize;

    if let Some(plan) = &app.plan {
        let selected_size = format_bytes(plan.summary.selected_size_bytes);
        lines.push(Line::from(vec![
            Span::styled(
                app.i18n.format(
                    "plan_candidates",
                    &[("count", plan.summary.candidate_count.to_string())],
                ),
                Style::default().fg(app.theme.fg_dim),
            ),
            Span::styled("  ·  ", Style::default().fg(app.theme.border)),
            Span::styled(
                app.i18n.format(
                    "plan_selected",
                    &[("count", plan.summary.selected_count.to_string())],
                ),
                Style::default().fg(app.theme.ok),
            ),
        ]));
        lines.push(Line::from(vec![Span::styled(
            app.i18n
                .format("plan_selected_size", &[("size", selected_size)]),
            Style::default()
                .fg(app.theme.cyan)
                .add_modifier(Modifier::BOLD),
        )]));
        lines.push(Line::from(""));

        if let Some(idx) = app.list_state.selected()
            && let Some(item) = plan.items.get(idx)
        {
            lines.push(Line::from(vec![Span::styled(
                app.i18n.t("plan_current_item"),
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            )]));
            let path_label = app.i18n.t("detail_path");
            let path_width = inner_width
                .saturating_sub(display_width(&path_label))
                .saturating_sub(2);
            lines.push(preview_field(
                path_label,
                truncate_text(&item.path.display().to_string(), path_width),
                app.theme.fg,
                app.theme,
            ));
            lines.push(preview_field(
                app.i18n.t("detail_size"),
                format_bytes(item.size_bytes),
                app.theme.cyan,
                app.theme,
            ));
            lines.push(preview_field(
                app.i18n.t("detail_rule"),
                item.rule_id.clone(),
                app.theme.fg,
                app.theme,
            ));
            lines.push(preview_field(
                app.i18n.t("detail_reason"),
                item.reason.clone(),
                app.theme.fg,
                app.theme,
            ));
            lines.push(preview_field(
                app.i18n.t("detail_risk"),
                item.risk_note.clone(),
                app.theme.warn,
                app.theme,
            ));
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

fn preview_field(label: String, value: String, value_color: Color, theme: Theme) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            label,
            Style::default()
                .fg(theme.accent)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(": "),
        Span::styled(value, Style::default().fg(value_color)),
    ])
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
