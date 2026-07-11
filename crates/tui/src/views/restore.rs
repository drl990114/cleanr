use super::*;

pub(crate) fn render_restore(frame: &mut Frame<'_>, area: Rect, app: &mut Workbench) {
    let manifests = &app.execution_manifests;
    let restored = restored_run_ids(&app.restore_manifests);
    let details = app
        .list_state
        .selected()
        .and_then(|idx| manifests.get(idx))
        .map_or_else(
            || {
                vec![
                    detail_line(
                        "Manifest dir",
                        app.state_dir.display().to_string(),
                        app.theme.fg_dim,
                        app.theme,
                    ),
                    detail_line(
                        "Runs",
                        manifests.len().to_string(),
                        app.theme.cyan,
                        app.theme,
                    ),
                ]
            },
            |manifest| {
                vec![
                    detail_line("Run", manifest.run_id.clone(), app.theme.cyan, app.theme),
                    detail_line(
                        "Restorable",
                        manifest.summary.succeeded.to_string(),
                        app.theme.ok,
                        app.theme,
                    ),
                    detail_line(
                        "Created",
                        manifest.created_at.to_rfc3339(),
                        app.theme.fg_dim,
                        app.theme,
                    ),
                    Line::from(""),
                    Line::from(Span::styled(
                        app.i18n.t("restore_select_hint"),
                        Style::default().fg(app.theme.fg_dim),
                    )),
                ]
            },
        );
    let title = app.i18n.t("label_restore");
    let detail_title = app.i18n.t("label_details");
    let empty_message = app.i18n.t("status_no_manifests");
    let restored_label = app.i18n.t("restore_state_restored");
    let available_label = app.i18n.t("restore_state_available");
    let item_count = manifests.len();
    let theme = app.theme;

    app.viewport_height = render_context_workspace_virtualized(
        frame,
        area,
        &mut app.list_state,
        theme,
        title,
        item_count,
        move |window| {
            if item_count == 0 {
                return vec![ListItem::new(Line::from(Span::styled(
                    empty_message,
                    Style::default().fg(theme.fg_dim),
                )))];
            }
            manifests[window]
                .iter()
                .map(|manifest| {
                    let was_restored = restored.contains(manifest.run_id.as_str());
                    let state = if was_restored {
                        restored_label.clone()
                    } else {
                        available_label.clone()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(manifest.run_id.clone(), Style::default().fg(theme.cyan)),
                        Span::styled(
                            format!("  {} item(s)", manifest.summary.succeeded),
                            Style::default().fg(theme.fg_dim),
                        ),
                        Span::styled(
                            format!("  {state}"),
                            Style::default().fg(if was_restored { theme.ok } else { theme.warn }),
                        ),
                    ]))
                })
                .collect()
        },
        detail_title,
        details,
    );
}
