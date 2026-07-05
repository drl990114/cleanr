use std::{
    collections::BTreeSet,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc::{self, Receiver, Sender},
    },
};

use anyhow::Result;
use cleanr_agent::{
    ActionRequest, AgentProvider, CleanupIntent, PathContext, PathInsight, create_agent,
    parse_slash_command,
};
use cleanr_config::{Config, default_config_path, default_state_dir};
use cleanr_core::{
    CleanupPlan, SafetyPolicy, ScanEntry, ScanRequest, ScanSummary, build_cleanup_plan_with_policy,
};
use cleanr_fs::{
    NO_GLOBAL_SCAN_ROOTS, SCAN_CANCELLED, ScanOptions, ScanPhase, ScanProgress, resolve_scan_roots,
};
use cleanr_i18n::I18n;
use cleanr_plugin_api::PluginDiagnostic;
use cleanr_rules::RuleRegistry;
use cleanr_tasks::{CleanupExecutor, TrashExecutor, restored_run_ids};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::widgets::ListState;

use crate::{
    commands::{command_name_for_status, filtered_palette_commands, palette_command_invocation},
    effects::{
        InsightEvent, TaskEvent, execute_cleanup, export_cleanup_plan, insight_channel,
        load_history, restore_cleanup, save_config, spawn_insight, spawn_scan,
    },
    theme::Theme,
    views::{format_bytes, usage_entries},
};

// -------------------------------------------------------------------------
// Application state
// -------------------------------------------------------------------------
pub(crate) enum Mode {
    Normal,
    Command,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) enum ConfirmChoice {
    Yes,
    #[default]
    No,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum View {
    Home,
    Scan,
    Languages,
    Rules,
    Plugins,
    Tasks,
    Usage,
    Restore,
}

#[derive(Clone, Debug, Default)]
pub(crate) enum InsightState {
    #[default]
    Empty,
    Loading,
    Ready(PathInsight),
    Error(String),
}

#[derive(Clone, Debug, Default)]
pub(crate) struct InsightPanel {
    pub(crate) target: Option<PathBuf>,
    pub(crate) state: InsightState,
}

pub struct Workbench {
    pub(crate) roots: Vec<PathBuf>,
    pub(crate) config: Config,
    pub(crate) registry: RuleRegistry,
    pub(crate) i18n: I18n,
    pub(crate) theme: Theme,
    pub(crate) agent: Box<dyn AgentProvider + Send>,
    pub(crate) state_dir: PathBuf,
    pub(crate) input: String,
    pub(crate) mode: Mode,
    pub(crate) view: View,
    pub(crate) palette_open: bool,
    pub(crate) help_open: bool,
    pub(crate) status: String,
    pub(crate) entries: Vec<ScanEntry>,
    pub(crate) scan_summary: ScanSummary,
    pub(crate) plan: Option<CleanupPlan>,
    pub(crate) task_log: Vec<String>,
    pub(crate) execution_manifests: Vec<cleanr_core::ExecutionManifest>,
    pub(crate) restore_manifests: Vec<cleanr_core::RestoreManifest>,
    pub(crate) scan_rx: Option<Receiver<TaskEvent>>,
    pub(crate) insight_rx: Option<Receiver<InsightEvent>>,
    pub(crate) insight_tx: Sender<InsightEvent>,
    pub(crate) insight: InsightPanel,
    pub(crate) scan_cancel: Option<Arc<AtomicBool>>,
    pub(crate) scan_progress: Option<ScanProgress>,
    pub(crate) review_after_scan: bool,
    pub(crate) usage_after_scan: bool,
    pub(crate) clean_waiting_for_confirmation: bool,
    pub(crate) restore_waiting_for_confirmation: Option<String>,
    pub(crate) confirm_choice: ConfirmChoice,
    pub(crate) should_quit: bool,
    pub(crate) list_state: ListState,
    pub(crate) palette_state: ListState,
    pub(crate) count_buffer: String,
    pub(crate) pending_key: Option<char>,
    pub(crate) viewport_height: u16,
    pub(crate) animation_tick: u64,
    pub(crate) ime_guard_phase: bool,
}

mod actions;
mod core;
mod input;
mod navigation;
mod tasks;
