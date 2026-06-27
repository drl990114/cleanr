use super::*;
use crate::{
    app::{ConfirmChoice, View},
    commands::palette_command_invocation,
    views::{
        bottom_bounded_rect, bounded_content_rect, centered_bounded_rect, command_cursor_position,
        ime_guard_position, render, usage_descendant_count,
    },
};
use cleanr_agent::{ActionRequest, CleanupIntent};
use cleanr_config::Config;
use cleanr_core::{
    Confidence, EXECUTION_SCHEMA_VERSION, EntryKind, ExecutionItem, ExecutionManifest,
    ExecutionStatus, ExecutionSummary, PlannedAction, RollbackReceipt, RuleHit, RuleTrust,
    ScanEntry,
};
use cleanr_fs::{ScanOptions, scan_paths};
use cleanr_i18n::{I18n, builtin_language_packs};
use cleanr_rules::RuleRegistry;
use cleanr_tasks::{FakeTrashExecutor, write_execution_manifest};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::{
    Terminal,
    backend::TestBackend,
    layout::{Position, Rect},
};
use std::{collections::BTreeMap, fs, path::PathBuf, thread, time::Duration};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::empty(),
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    }
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: crossterm::event::KeyEventState::empty(),
    }
}

fn app(root: PathBuf) -> Workbench {
    Workbench::new(
        vec![root],
        Config::default(),
        RuleRegistry::builtin().expect("builtin rules"),
        I18n::new("en-US", BTreeMap::new(), builtin_language_packs()),
        Theme::dark(),
    )
    .expect("create workbench")
}

fn render_text(app: &mut Workbench, width: u16, height: u16) -> String {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    terminal
        .draw(|frame| render(frame, app))
        .expect("render frame");
    let buffer = terminal.backend().buffer();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn test_rule_hit(rule_id: &str) -> RuleHit {
    RuleHit {
        rule_pack_id: "builtin-dev".into(),
        rule_id: rule_id.into(),
        label: "Generated".into(),
        category: "build-cache".into(),
        confidence: Confidence::High,
        reason: "generated".into(),
        risk_note: "rebuild".into(),
        default_selected: true,
        trust: RuleTrust::Builtin,
    }
}

#[test]
fn starts_in_workbench_with_empty_command_input() {
    let temp = tempfile::tempdir().expect("tempdir");
    let app = app(temp.path().to_path_buf());
    assert_eq!(app.input(), "");
    assert!(!app.palette_open());
    assert!(app.status().contains("Ready"));
}

#[test]
fn agent_cannot_execute_when_confirmation_dialog_is_disabled() {
    let temp = tempfile::tempdir().expect("tempdir");
    let target = temp.path().join("target");
    fs::create_dir(&target).expect("mkdir");
    fs::write(target.join("artifact"), vec![0; 2 * 1024 * 1024]).expect("write artifact");
    let mut app = app(temp.path().to_path_buf());
    app.config.cleanup.require_confirm = false;
    let report = scan_paths(&app.roots, &ScanOptions::default()).expect("scan");
    app.entries = report.entries;
    app.registry.annotate_entries(&mut app.entries);
    app.build_plan();
    let executor = FakeTrashExecutor::default();

    app.clean_with_executor(CleanupIntent::AgentRequest, &executor);

    assert!(executor.trashed_paths().is_empty());
    assert!(target.exists());
    assert!(app.clean_waiting_for_confirmation);
    assert!(app.status().contains("Review plan"));
}

#[test]
fn home_layout_has_one_clear_primary_action() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    let screen = render_text(&mut app, 100, 28);
    println!("{screen}");

    assert!(screen.contains("Safe intelligent disk organization"));
    assert!(screen.contains("Scan & analyze"));
    assert!(screen.contains("Every item is reviewed first"));
    assert!(!screen.contains("Recent activity"));
    assert!(!screen.contains("No scan yet"));
}

#[test]
fn home_layout_switches_to_a_concise_scan_result() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(
        temp.path().join("node_modules").join("index.js"),
        vec![0; 2 * 1024 * 1024],
    )
    .expect("write");

    let mut app = app(temp.path().to_path_buf());
    let mut report =
        scan_paths(&[temp.path().to_path_buf()], &ScanOptions::default()).expect("scan");
    app.registry.annotate_entries(&mut report.entries);
    app.entries = report.entries;
    app.scan_summary = report.summary;
    app.build_plan_for_view(false);

    let screen = render_text(&mut app, 100, 28);
    println!("{screen}");

    assert!(screen.contains("Scan result"));
    assert!(screen.contains("Reclaimable"));
    assert!(screen.contains("Review cleanup items"));
    assert!(!screen.contains("Recent activity"));
}

#[test]
fn chinese_home_matches_the_primary_terminal_layout() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = Workbench::new(
        vec![temp.path().to_path_buf()],
        Config::default(),
        RuleRegistry::builtin().expect("builtin rules"),
        I18n::new("zh-CN", BTreeMap::new(), builtin_language_packs()),
        Theme::dark(),
    )
    .expect("create workbench");

    let screen = render_text(&mut app, 143, 41);
    let compact = screen
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>();

    assert!(compact.contains("安全智能磁盘整理"));
    assert!(compact.contains("扫描分析"));
    assert!(compact.contains("所有清理项都会先审阅"));
    assert!(!compact.contains("最近活动"));
    assert!(!compact.contains("尚未扫描"));
}

#[test]
fn single_key_shortcuts_start_primary_actions() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());

    app.handle_key(key(KeyCode::Char('s')));
    assert!(app.is_scan_running());
    assert_eq!(app.view, View::Scan);
}

#[test]
fn scan_layout_keeps_selection_and_details_distinct() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(
        temp.path().join("node_modules").join("index.js"),
        vec![0; 2 * 1024 * 1024],
    )
    .expect("write");

    let mut app = app(temp.path().to_path_buf());
    let mut report =
        scan_paths(&[temp.path().to_path_buf()], &ScanOptions::default()).expect("scan");
    app.registry.annotate_entries(&mut report.entries);
    app.entries = report.entries;
    app.scan_summary = report.summary;
    app.build_plan();

    let screen = render_text(&mut app, 120, 30);
    println!("{screen}");

    assert!(screen.contains("[✓]"));
    assert!(screen.contains("Preview"));
    assert!(screen.contains("space select"));
}

#[test]
fn slash_opens_command_palette() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.handle_key(key(KeyCode::Char('/')));
    assert!(app.palette_open());
    assert_eq!(app.input(), "/");
}

#[test]
fn command_palette_tabs_wrap_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.handle_key(key(KeyCode::Char('/')));
    let len = app.filtered_palette_commands().len();

    app.handle_key(key(KeyCode::BackTab));
    assert_eq!(app.palette_state.selected(), Some(len - 1));

    app.handle_key(key(KeyCode::Tab));
    assert_eq!(app.palette_state.selected(), Some(0));
}

#[test]
fn confirmation_supports_arrows_and_y_n_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.clean_waiting_for_confirmation = true;
    app.confirm_choice = ConfirmChoice::No;

    app.handle_key(key(KeyCode::Left));
    assert_eq!(app.confirm_choice, ConfirmChoice::Yes);
    assert!(app.clean_waiting_for_confirmation);

    app.handle_key(key(KeyCode::Right));
    assert_eq!(app.confirm_choice, ConfirmChoice::No);

    app.handle_key(key(KeyCode::Char('y')));
    assert_eq!(app.confirm_choice, ConfirmChoice::Yes);

    app.handle_key(key(KeyCode::Char('n')));
    assert_eq!(app.confirm_choice, ConfirmChoice::No);
}

#[test]
fn confirmation_enter_submits_current_choice() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.clean_waiting_for_confirmation = true;
    app.confirm_choice = ConfirmChoice::No;

    app.handle_key(key(KeyCode::Enter));

    assert!(!app.clean_waiting_for_confirmation);
    assert!(app.status().contains("cancelled"));
}

#[test]
fn confirmation_escape_always_cancels() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.clean_waiting_for_confirmation = true;
    app.confirm_choice = ConfirmChoice::Yes;

    app.handle_key(key(KeyCode::Esc));

    assert!(!app.clean_waiting_for_confirmation);
    assert_eq!(app.confirm_choice, ConfirmChoice::No);
}

#[test]
fn confirmation_dialog_renders_in_small_terminals() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.clean_waiting_for_confirmation = true;
    app.confirm_choice = ConfirmChoice::No;

    for (width, height) in [(40, 10), (80, 24), (194, 64)] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render");
    }
}

#[test]
fn restore_view_requests_confirmation_for_selected_run() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.state_dir = temp.path().to_path_buf();
    write_execution_manifest(
        &ExecutionManifest {
            schema_version: EXECUTION_SCHEMA_VERSION.to_string(),
            run_id: "restore-test".to_string(),
            created_at: chrono::Utc::now(),
            plan_schema_version: "plan".to_string(),
            summary: ExecutionSummary {
                attempted: 1,
                succeeded: 1,
                failed: 0,
            },
            items: vec![ExecutionItem {
                path: temp.path().join("target"),
                planned_action: PlannedAction::Trash,
                status: ExecutionStatus::Trashed,
                rule_id: "test".to_string(),
                rollback_receipt: Some(RollbackReceipt {
                    method: "fake".to_string(),
                    note: "test".to_string(),
                    locator: Some("fake".to_string()),
                }),
                error: None,
            }],
        },
        &app.state_dir,
    )
    .expect("write manifest");

    app.dispatch(ActionRequest::Restore);
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(
        app.restore_waiting_for_confirmation.as_deref(),
        Some("restore-test")
    );
}

#[test]
fn scan_command_runs_in_background_and_finds_candidates() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(
        temp.path().join("node_modules").join("a.js"),
        vec![0; 2 * 1024 * 1024],
    )
    .expect("write");

    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Scan(Vec::new()));
    assert!(app.is_scan_running());
    app.handle_key(key(KeyCode::Char('/')));
    assert_eq!(app.input(), "/");

    for _ in 0..50 {
        app.poll_tasks();
        if !app.is_scan_running() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    assert!(!app.is_scan_running());
    app.dispatch(ActionRequest::Review);
    assert_eq!(app.plan().expect("plan").summary.selected_count, 1);
}

#[test]
fn scan_view_can_render_selection_beyond_old_candidate_cap() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.entries = (0..501)
        .map(|index| ScanEntry {
            path: temp.path().join(format!("candidate-{index:03}")),
            kind: EntryKind::File,
            size_bytes: 1,
            modified_at: None,
            rule_hits: vec![test_rule_hit("generated")],
        })
        .collect();
    app.build_plan();
    app.list_state.select(Some(500));
    let selected_name = app.plan().expect("plan").items[500]
        .path
        .file_name()
        .expect("file name")
        .to_string_lossy()
        .into_owned();

    let screen = render_text(&mut app, 120, 24);

    assert!(screen.contains(&selected_name), "{screen}");
}

#[test]
fn restore_view_can_render_selection_beyond_old_history_cap() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.execution_manifests = (0..21)
        .map(|index| ExecutionManifest {
            schema_version: EXECUTION_SCHEMA_VERSION.to_string(),
            run_id: format!("run-{index:02}"),
            created_at: chrono::Utc::now(),
            plan_schema_version: "plan".to_string(),
            summary: ExecutionSummary {
                attempted: 1,
                succeeded: 1,
                failed: 0,
            },
            items: vec![],
        })
        .collect();
    app.view = View::Restore;
    app.list_state.select(Some(20));

    let screen = render_text(&mut app, 120, 24);

    assert!(screen.contains("run-20"), "{screen}");
}

#[test]
fn cleanup_success_starts_background_refresh_scan() {
    let temp = tempfile::tempdir().expect("tempdir");
    let state_dir = temp.path().join("state");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(
        temp.path().join("node_modules").join("index.js"),
        vec![0; 2 * 1024 * 1024],
    )
    .expect("write");
    let mut app = app(temp.path().to_path_buf());
    app.state_dir = state_dir;
    let report = scan_paths(&app.roots, &ScanOptions::default()).expect("scan");
    app.entries = report.entries;
    app.registry.annotate_entries(&mut app.entries);
    app.build_plan();
    let executor = FakeTrashExecutor::default();

    app.clean_with_executor(CleanupIntent::ExplicitUserConfirmation, &executor);

    assert!(app.is_scan_running());
}

#[test]
fn usage_scan_exposes_live_progress() {
    let temp = tempfile::tempdir().expect("tempdir");
    for index in 0..128 {
        fs::write(temp.path().join(format!("file-{index}")), b"1234").expect("write");
    }

    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Usage(Vec::new()));
    assert_eq!(app.view, View::Usage);

    for _ in 0..50 {
        app.poll_tasks();
        if app
            .scan_progress
            .as_ref()
            .is_some_and(|progress| progress.entries_total > 0)
            || !app.is_scan_running()
        {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }

    assert!(
        app.scan_progress
            .as_ref()
            .is_some_and(|progress| progress.entries_total > 0)
            || !app.is_scan_running()
    );
}

#[test]
fn adaptive_rects_never_exceed_terminal_area() {
    let area = Rect::new(3, 5, 40, 10);
    let content = bounded_content_rect(area, 164, 30);
    let popup = centered_bounded_rect(area, 100, 40, 120);
    let bottom = bottom_bounded_rect(area, 100, 40, 120);

    for rect in [content, popup, bottom] {
        assert!(rect.x >= area.x);
        assert!(rect.y >= area.y);
        assert!(rect.right() <= area.right());
        assert!(rect.bottom() <= area.bottom());
    }
}

#[test]
fn usage_renders_at_small_and_large_terminal_sizes() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("target")).expect("mkdir");
    fs::write(temp.path().join("target").join("artifact"), vec![0; 4096]).expect("write");
    let mut app = app(temp.path().to_path_buf());
    let report = scan_paths(&[temp.path().to_path_buf()], &ScanOptions::default()).expect("scan");
    app.entries = report.entries;
    app.scan_summary = report.summary;
    app.show_usage();

    for (width, height) in [(40, 10), (80, 24), (194, 64)] {
        let backend = TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        terminal
            .draw(|frame| render(frame, &mut app))
            .expect("render");
    }
}

#[test]
fn usage_details_count_recursive_directory_entries() {
    let root = PathBuf::from("/workspace/target");
    let entries = vec![
        ScanEntry {
            path: root.clone(),
            kind: EntryKind::Directory,
            size_bytes: 10,
            modified_at: None,
            rule_hits: vec![],
        },
        ScanEntry {
            path: root.join("debug"),
            kind: EntryKind::Directory,
            size_bytes: 10,
            modified_at: None,
            rule_hits: vec![],
        },
        ScanEntry {
            path: root.join("debug/app"),
            kind: EntryKind::File,
            size_bytes: 10,
            modified_at: None,
            rule_hits: vec![],
        },
        ScanEntry {
            path: PathBuf::from("/workspace/README.md"),
            kind: EntryKind::File,
            size_bytes: 1,
            modified_at: None,
            rule_hits: vec![],
        },
    ];

    assert_eq!(usage_descendant_count(&entries, &entries[0]), 2);
    assert_eq!(usage_descendant_count(&entries, &entries[2]), 0);
}

#[test]
fn ime_guard_stays_inside_terminal_and_off_the_last_row() {
    for area in [
        Rect::new(0, 0, 80, 24),
        Rect::new(3, 5, 40, 10),
        Rect::new(0, 0, 1, 1),
    ] {
        let position = ime_guard_position(area);
        assert!(position.x >= area.x);
        assert!(position.y >= area.y);
        assert!(position.x < area.right().max(area.x + 1));
        assert!(position.y < area.bottom().max(area.y + 1));
        if area.height > 1 {
            assert!(position.y < area.bottom().saturating_sub(1));
        }
    }
}

#[test]
fn command_cursor_accounts_for_wide_chinese_input() {
    let area = Rect::new(1, 20, 40, 1);
    assert_eq!(
        command_cursor_position(area, ":中文"),
        Some(Position::new(8, 20))
    );
}

#[test]
fn languages_command_reports_loaded_packs() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Languages);
    assert_eq!(app.view, View::Languages);
    assert_eq!(app.list_state.selected(), Some(0));
    assert!(app.status().contains("Active locale"));
}

#[test]
fn context_views_support_arrow_navigation() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());

    app.dispatch(ActionRequest::Languages);
    assert_eq!(app.list_state.selected(), Some(0));

    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.list_state.selected(), Some(1));

    app.handle_key(key(KeyCode::Up));
    assert_eq!(app.list_state.selected(), Some(0));
}

#[test]
fn home_shortcuts_hide_context_views() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());

    assert!(app.is_home());

    app.dispatch(ActionRequest::Languages);
    assert_eq!(app.view, View::Languages);

    app.handle_key(key(KeyCode::Char('h')));
    assert!(app.is_home());

    app.dispatch(ActionRequest::Usage(Vec::new()));
    assert_eq!(app.view, View::Usage);

    app.handle_key(key(KeyCode::Esc));
    for _ in 0..50 {
        app.poll_tasks();
        if !app.is_scan_running() {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    app.handle_key(key(KeyCode::Esc));
    assert!(app.is_home());
}

#[test]
fn toggling_selection_updates_summary() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(
        temp.path().join("node_modules").join("a.js"),
        vec![0; 2 * 1024 * 1024],
    )
    .expect("write");

    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Scan(Vec::new()));
    for _ in 0..50 {
        app.poll_tasks();
        if !app.is_scan_running() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let plan = app.plan().expect("plan");
    let initial = plan.summary.selected_count;
    app.handle_key(key(KeyCode::Char(' ')));
    let plan = app.plan().expect("plan");
    assert_ne!(plan.summary.selected_count, initial);
}

#[test]
fn palette_selection_dispatches_non_scan_command() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());

    app.handle_key(key(KeyCode::Char('/')));
    assert!(app.palette_open());

    // Navigate to /languages in the palette.
    for _ in 0..5 {
        app.handle_key(key(KeyCode::Down));
    }
    app.handle_key(key(KeyCode::Enter));

    assert!(!app.palette_open());
    assert!(app.status().contains("Active locale"));
}

#[test]
fn palette_enter_dispatches_filtered_selection() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());

    app.handle_key(key(KeyCode::Char('/')));
    for ch in "langu".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Enter));

    assert_eq!(app.view, View::Languages);
    assert!(app.status().contains("Active locale"));
}

#[test]
fn palette_invocation_keeps_flags_and_drops_placeholders() {
    assert_eq!(
        palette_command_invocation("/clean --confirm"),
        "/clean --confirm"
    );
    assert_eq!(palette_command_invocation("/scan [path...]"), "/scan");
    assert_eq!(
        palette_command_invocation("/export-plan [path]"),
        "/export-plan"
    );
}

#[test]
fn command_mode_ctrl_w_deletes_word_and_ctrl_u_clears_line() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.handle_key(key(KeyCode::Char('/')));
    for ch in "scan /tmp".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    assert_eq!(app.input(), "/scan /tmp");

    app.handle_key(ctrl(KeyCode::Char('w')));
    assert_eq!(app.input(), "/scan ");

    app.handle_key(ctrl(KeyCode::Char('u')));
    assert_eq!(app.input(), "/");
}

#[test]
fn gg_goto_first_and_goto_last() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Rules);
    app.handle_key(key(KeyCode::Down));
    app.handle_key(key(KeyCode::Down));
    assert_eq!(app.list_state.selected(), Some(2));

    app.handle_key(key(KeyCode::Char('g')));
    app.handle_key(key(KeyCode::Char('g')));
    assert_eq!(app.list_state.selected(), Some(0));

    app.handle_key(key(KeyCode::Char('G')));
    assert_eq!(app.list_state.selected(), Some(app.list_len() - 1));
}

#[test]
fn count_prefix_moves_multiple_lines() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Rules);
    assert!(app.list_len() >= 3);

    for ch in "2".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Char('j')));
    assert_eq!(app.list_state.selected(), Some(2));
}

#[test]
fn count_prefix_goto_line() {
    let temp = tempfile::tempdir().expect("tempdir");
    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Rules);

    for ch in "2".chars() {
        app.handle_key(key(KeyCode::Char(ch)));
    }
    app.handle_key(key(KeyCode::Char('G')));
    assert_eq!(app.list_state.selected(), Some(1));
}

#[test]
fn toggle_all_selects_and_deselects_scan_items() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir(temp.path().join("node_modules")).expect("mkdir");
    fs::write(
        temp.path().join("node_modules").join("a.js"),
        vec![0; 2 * 1024 * 1024],
    )
    .expect("write");

    let mut app = app(temp.path().to_path_buf());
    app.dispatch(ActionRequest::Scan(Vec::new()));
    for _ in 0..50 {
        app.poll_tasks();
        if !app.is_scan_running() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }

    let plan = app.plan().expect("plan");
    let initial = plan.summary.selected_count;

    app.handle_key(key(KeyCode::Char('a')));
    let plan = app.plan().expect("plan");
    assert_ne!(plan.summary.selected_count, initial);

    app.handle_key(key(KeyCode::Char('%')));
    let plan = app.plan().expect("plan");
    assert_eq!(plan.summary.selected_count, initial);
}
