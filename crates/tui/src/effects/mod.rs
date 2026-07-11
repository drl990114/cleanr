use std::{
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::AtomicBool,
        mpsc::{self, Receiver},
    },
};

use anyhow::{Context, Result};
use cleanr_config::Config;
use cleanr_core::{CleanupPlan, ExecutionManifest, RestoreManifest};
use cleanr_fs::{ScanOptions, ScanProgress, ScanReport, scan_paths_with_progress_cancellable};
use cleanr_i18n::I18n;
use cleanr_plugin_api::discover_bundles;
use cleanr_rules::RuleRegistry;
use cleanr_tasks::{
    CleanupAuthorization, CleanupExecutor, ManifestRepository, SystemRestoreExecutor,
    execute_cleanup_plan, restore_execution_manifest, write_cleanup_plan,
};

pub(crate) enum TaskEvent {
    ScanProgress(ScanProgress),
    ScanFinished(std::result::Result<ScanReport, String>),
}

pub(crate) struct ScanEffect {
    pub receiver: Receiver<TaskEvent>,
    pub cancellation: Arc<AtomicBool>,
}

pub(crate) fn load_runtime(config: &Config) -> Result<(RuleRegistry, I18n)> {
    let discovery = discover_bundles(
        &config.plugins.dirs,
        &config.plugins.trusted,
        env!("CARGO_PKG_VERSION"),
    );
    Ok((
        RuleRegistry::load_with_discovery(config, &discovery)?,
        I18n::load_with_discovery(config, &discovery)?,
    ))
}

pub(crate) fn spawn_scan(roots: Vec<PathBuf>, options: ScanOptions) -> Result<ScanEffect> {
    let (sender, receiver) = mpsc::channel();
    let cancellation = Arc::new(AtomicBool::new(false));
    let worker_cancellation = Arc::clone(&cancellation);
    std::thread::Builder::new()
        .name("cleanr-scan".to_string())
        .spawn(move || {
            let result = scan_paths_with_progress_cancellable(
                &roots,
                &options,
                &worker_cancellation,
                |progress| {
                    let _ = sender.send(TaskEvent::ScanProgress(progress));
                },
            )
            .map_err(|error| error.to_string());
            let _ = sender.send(TaskEvent::ScanFinished(result));
        })
        .context("failed to spawn scan worker")?;
    Ok(ScanEffect {
        receiver,
        cancellation,
    })
}

pub(crate) fn load_history(
    state_dir: &Path,
) -> Result<(Vec<ExecutionManifest>, Vec<RestoreManifest>)> {
    ManifestRepository::new(state_dir).history()
}

pub(crate) fn execute_cleanup(
    plan: &CleanupPlan,
    executor: &impl CleanupExecutor,
    state_dir: &Path,
    user_authorized: bool,
) -> Result<ExecutionManifest> {
    let authorization = user_authorized.then(CleanupAuthorization::explicit_user_confirmation);
    execute_cleanup_plan(plan, executor, state_dir, authorization.as_ref())
}

pub(crate) fn restore_cleanup(
    manifest: &ExecutionManifest,
    state_dir: &Path,
) -> Result<RestoreManifest> {
    restore_execution_manifest(manifest, &SystemRestoreExecutor, state_dir)
}

pub(crate) fn export_cleanup_plan(plan: &CleanupPlan, path: &Path) -> Result<()> {
    write_cleanup_plan(plan, path)
}

pub(crate) fn save_config(config: &Config, path: &Path) -> Result<()> {
    config.save_to(path)
}
