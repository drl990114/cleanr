use super::*;

pub(crate) fn render_command(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    let content = match app.mode {
        Mode::Command => {
            let prefix = app.input.chars().next().unwrap_or('>');
            let rest = &app.input[prefix.len_utf8()..];
            Line::from(vec![
                Span::styled(
                    format!(" {prefix} "),
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(rest.to_string(), Style::default().fg(app.theme.fg)),
                Span::styled("▏", Style::default().fg(app.theme.accent)),
            ])
        }
        Mode::Normal => Line::from(vec![
            Span::styled(" / ", Style::default().fg(app.theme.accent)),
            Span::styled(
                app.i18n.t("command_placeholder"),
                Style::default().fg(app.theme.fg_dim),
            ),
        ]),
    };

    let content_area = bounded_content_rect(area, 164, area.height);
    let block = Block::default()
        .borders(Borders::TOP)
        .border_style(Style::default().fg(app.theme.border))
        .padding(Padding::horizontal(1));
    let inner = block.inner(content_area);
    frame.render_widget(Paragraph::new(content).block(block), content_area);

    if matches!(app.mode, Mode::Command)
        && let Some(position) = command_cursor_position(inner, &app.input)
    {
        frame.set_cursor_position(position);
    }
}

pub(crate) fn render_status(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    frame.render_widget(
        Block::default().style(Style::default().bg(app.theme.surface)),
        area,
    );
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(72), Constraint::Percentage(28)])
        .split(area);
    let mode = match app.mode {
        Mode::Normal => app.i18n.t("label_mode_normal"),
        Mode::Command => app.i18n.t("label_mode_command"),
    };
    let mode_color = match app.mode {
        Mode::Normal => app.theme.accent,
        Mode::Command => app.theme.magenta,
    };
    let mut hints = vec![
        Span::styled(
            format!("  {mode}"),
            Style::default().fg(mode_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  ·  ", Style::default().fg(app.theme.border)),
    ];
    match app.mode {
        Mode::Command => {
            hints.extend(key_hint("↑↓", app.i18n.t("hint_choose"), app.theme));
            hints.extend(key_hint("enter", app.i18n.t("hint_run"), app.theme));
            hints.extend(key_hint("esc", app.i18n.t("hint_close"), app.theme));
        }
        Mode::Normal => {
            if app.is_scan_running() {
                hints.extend(key_hint("esc/x", app.i18n.t("hint_cancel"), app.theme));
            } else if app.view == View::Home {
                hints.extend(key_hint("s", app.i18n.t("hint_scan"), app.theme));
                hints.extend(key_hint("u", app.i18n.t("hint_usage"), app.theme));
            } else if app.view == View::Scan {
                if app.list_len() > 0 {
                    hints.extend(key_hint("j/k", app.i18n.t("hint_move"), app.theme));
                    hints.extend(key_hint("space", app.i18n.t("hint_select"), app.theme));
                    if app.plan.is_some() && area.width >= 88 {
                        hints.extend(key_hint("i", app.i18n.t("hint_inspect"), app.theme));
                    }
                    if app.plan.is_some() && area.width >= 104 {
                        hints.extend(key_hint("a", app.i18n.t("hint_all"), app.theme));
                    }
                    if app.plan.is_some() && area.width >= 96 {
                        hints.extend(key_hint("c", app.i18n.t("hint_clean"), app.theme));
                    }
                }
            } else if app.list_len() > 0 {
                hints.extend(key_hint("j/k", app.i18n.t("hint_move"), app.theme));
                if matches!(app.view, View::Languages | View::Restore) {
                    hints.extend(key_hint("enter", app.i18n.t("hint_select"), app.theme));
                }
            }
            hints.extend(key_hint("/", app.i18n.t("hint_commands"), app.theme));
            if area.width >= 120 {
                hints.extend(key_hint("?", app.i18n.t("hint_help"), app.theme));
            }
            hints.extend(key_hint("q", app.i18n.t("hint_quit"), app.theme));
        }
    }
    frame.render_widget(Paragraph::new(Line::from(hints)), chunks[0]);

    let mut right = Vec::new();
    if app.is_scan_running() {
        let progress = app.scan_progress.as_ref();
        let compact = progress.map_or_else(
            || app.i18n.t("scan_preparing"),
            |value| match value.phase {
                ScanPhase::Discovering => app.i18n.format(
                    "scan_progress_discovered",
                    &[("total", value.entries_total.to_string())],
                ),
                ScanPhase::Scanning if value.entries_total == 0 => app.i18n.format(
                    "scan_progress_unbounded",
                    &[("scanned", value.entries_scanned.to_string())],
                ),
                ScanPhase::Scanning => app.i18n.format(
                    "scan_progress_count",
                    &[
                        ("scanned", value.entries_scanned.to_string()),
                        ("total", value.entries_total.to_string()),
                    ],
                ),
                ScanPhase::Aggregating => app.i18n.t("scan_progress_aggregating"),
            },
        );
        right.push(Span::styled(
            format!("{} {compact}", spinner_frame(app.animation_tick)),
            Style::default().fg(app.theme.warn),
        ));
    } else if let Some(plan) = &app.plan {
        right.extend([
            Span::styled(
                plan.summary.selected_count.to_string(),
                Style::default()
                    .fg(app.theme.ok)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" / {} selected ", plan.summary.candidate_count),
                Style::default().fg(app.theme.fg_dim),
            ),
        ]);
    } else if app.list_len() > 0 {
        let current = app.list_state.selected().map_or(0, |index| index + 1);
        right.push(Span::styled(
            format!("{current} / {} ", app.list_len()),
            Style::default().fg(app.theme.fg_dim),
        ));
    }
    frame.render_widget(
        Paragraph::new(Line::from(right)).alignment(ratatui::layout::Alignment::Right),
        chunks[1],
    );
}

pub(crate) fn render_palette(frame: &mut Frame<'_>, area: Rect, app: &mut Workbench) {
    frame.render_widget(Clear, area);
    let filter = app
        .input
        .strip_prefix('/')
        .unwrap_or("")
        .trim()
        .to_lowercase();

    let commands = app.filtered_palette_commands();
    let items = commands
        .iter()
        .map(|command| {
            let translated = app.i18n.t(command.description_key);
            let description = if translated == command.description_key {
                command.description.to_string()
            } else {
                translated
            };

            let command_padding = " ".repeat(24usize.saturating_sub(command.name.len()));
            let mut spans = vec![
                Span::styled(command.name, Style::default().fg(app.theme.accent)),
                Span::raw(command_padding.clone()),
                Span::styled(description.clone(), Style::default().fg(app.theme.fg_dim)),
            ];

            // Highlight matching characters in the command name.
            if !filter.is_empty() {
                let name_lower = command.name.to_lowercase();
                if let Some(start) = name_lower.find(&filter) {
                    let end = start + filter.len();
                    let before = &command.name[..start];
                    let matched = &command.name[start..end];
                    let after = &command.name[end..];
                    spans = vec![
                        Span::raw(before.to_string()),
                        Span::styled(
                            matched.to_string(),
                            Style::default()
                                .fg(app.theme.warn)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::raw(after.to_string()),
                        Span::raw(command_padding),
                        Span::styled(description.clone(), Style::default().fg(app.theme.fg_dim)),
                    ];
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect::<Vec<_>>();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::TOP | Borders::BOTTOM)
                .border_style(Style::default().fg(app.theme.border))
                .padding(Padding::horizontal(1))
                .style(Style::default().bg(app.theme.surface))
                .title(format!(" {} ", app.i18n.t("label_slash_commands")))
                .title_style(
                    Style::default()
                        .fg(app.theme.accent)
                        .add_modifier(Modifier::BOLD),
                ),
        )
        .highlight_style(
            Style::default()
                .bg(app.theme.surface_alt)
                .fg(app.theme.highlight_fg),
        )
        .highlight_symbol("  ");

    frame.render_stateful_widget(list, area, &mut app.palette_state);
}

pub(crate) fn render_help(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    frame.render_widget(Clear, area);
    let lines = vec![
        Line::from(vec![Span::styled(
            app.i18n.t("help_title"),
            Style::default()
                .fg(app.theme.accent)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(app.i18n.t("help_move")),
        Line::from(app.i18n.t("help_select_all")),
        Line::from(app.i18n.t("help_toggle")),
        Line::from(app.i18n.t("help_actions")),
        Line::from(app.i18n.t("help_inspect")),
        Line::from(app.i18n.t("help_command")),
        Line::from(app.i18n.t("help_palette")),
        Line::from(app.i18n.t("help_page")),
        Line::from(app.i18n.t("help_home")),
        Line::from(app.i18n.t("help_confirm_yes")),
        Line::from(app.i18n.t("help_confirm_no")),
        Line::from(app.i18n.t("help_quit")),
    ];
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(app.theme.border))
            .padding(Padding::horizontal(2))
            .style(Style::default().bg(app.theme.surface))
            .title(format!(" {} ", app.i18n.t("label_help")))
            .title_style(
                Style::default()
                    .fg(app.theme.accent)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(paragraph, area);
}

pub(crate) fn render_confirm(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    frame.render_widget(Clear, area);
    let restoring = app.restore_waiting_for_confirmation.is_some();
    let (title, body, action_color) = if restoring {
        let run_id = app
            .restore_waiting_for_confirmation
            .as_deref()
            .unwrap_or_default();
        let count = app
            .execution_manifests
            .iter()
            .find(|manifest| manifest.run_id == run_id)
            .map_or(0, |manifest| manifest.summary.succeeded);
        (
            app.i18n.t("confirm_restore_title"),
            app.i18n.format(
                "confirm_restore_body",
                &[("count", count.to_string()), ("run_id", run_id.to_string())],
            ),
            app.theme.ok,
        )
    } else {
        let (count, size) = app.plan.as_ref().map_or((0, String::from("-")), |plan| {
            (
                plan.summary.selected_count,
                format_bytes(plan.summary.selected_size_bytes),
            )
        });
        (
            app.i18n.t("confirm_title"),
            app.i18n.format(
                "confirm_body",
                &[("count", count.to_string()), ("size", size)],
            ),
            app.theme.danger,
        )
    };

    let lines = vec![
        Line::from(""),
        Line::from(body).alignment(ratatui::layout::Alignment::Center),
        Line::from(""),
        Line::from(vec![
            confirm_button(
                "Y",
                app.i18n.t("confirm_yes"),
                app.confirm_choice == ConfirmChoice::Yes,
                action_color,
                app.theme,
            ),
            Span::raw("   "),
            confirm_button(
                "N",
                app.i18n.t("confirm_no"),
                app.confirm_choice == ConfirmChoice::No,
                app.theme.accent,
                app.theme,
            ),
        ])
        .alignment(ratatui::layout::Alignment::Center),
        Line::from(Span::styled(
            app.i18n.t("confirm_hint"),
            Style::default().fg(app.theme.fg_dim),
        ))
        .alignment(ratatui::layout::Alignment::Center),
    ];
    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::TOP | Borders::BOTTOM)
            .border_style(Style::default().fg(app.theme.border))
            .padding(Padding::horizontal(2))
            .style(Style::default().bg(app.theme.surface))
            .title(format!(" {title} "))
            .title_style(
                Style::default()
                    .fg(action_color)
                    .add_modifier(Modifier::BOLD),
            ),
    );
    frame.render_widget(paragraph, area);
}

pub(crate) fn confirm_button(
    shortcut: &'static str,
    label: String,
    selected: bool,
    selected_color: Color,
    theme: Theme,
) -> Span<'static> {
    let style = if selected {
        Style::default()
            .bg(selected_color)
            .fg(theme.highlight_fg)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(theme.fg_dim)
    };
    let shortcut = if selected {
        format!("[{shortcut}]")
    } else {
        format!("({shortcut})")
    };
    Span::styled(format!("  {shortcut} {label}  "), style)
}

pub(crate) fn render_ime_guard(frame: &mut Frame<'_>, area: Rect, app: &Workbench) {
    if area.is_empty() {
        return;
    }
    let position = ime_guard_position(area);
    let style = if app.ime_guard_phase {
        Style::default().bg(app.theme.bg)
    } else {
        Style::default()
            .bg(app.theme.bg)
            .add_modifier(Modifier::DIM)
    };
    frame.render_widget(
        Paragraph::new(" ").style(style),
        Rect::new(position.x, position.y, 1, 1),
    );
}
