use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use cleanr_config::{Config, default_config_path, default_state_dir};
use cleanr_core::{CleanupPlan, SafetyPolicy, build_cleanup_plan_with_policy};
use cleanr_fs::{ScanOptions, ScanReport, developer_cache_roots, scan_paths};
use cleanr_rules::RuleRegistry;
use cleanr_tasks::{
    ManifestRepository, SystemRestoreExecutor, restore_execution_manifest, restored_run_ids,
    write_cleanup_plan,
};
use serde_json::json;

pub struct ScanCommand {
    pub config_path: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub include_global: bool,
    pub json: bool,
}

pub struct PlanCommand {
    pub config_path: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub include_global: bool,
    pub output: Option<PathBuf>,
}

pub struct DryRunCommand {
    pub config_path: Option<PathBuf>,
    pub paths: Vec<PathBuf>,
    pub include_global: bool,
    pub json: bool,
    pub output: Option<PathBuf>,
}

struct WorkflowScan {
    config: Config,
    config_path: Option<PathBuf>,
    registry: RuleRegistry,
    roots: Vec<PathBuf>,
    report: ScanReport,
}

pub fn scan(command: ScanCommand) -> Result<()> {
    let scan = run_scan(command.config_path, command.paths, command.include_global)?;
    if command.json {
        print_scan_json(&scan.report)?;
    } else {
        let candidates = scan
            .report
            .entries
            .iter()
            .filter(|entry| !entry.rule_hits.is_empty())
            .count();
        println!(
            "Scanned {} entrie(s), found {} candidate(s), total size {}.",
            scan.report.summary.entries_seen,
            candidates,
            format_bytes(scan.report.summary.total_size_bytes)
        );
        if scan.report.summary.errors > 0 {
            println!("Scan errors: {}", scan.report.summary.errors);
        }
    }
    Ok(())
}

pub fn plan(command: PlanCommand) -> Result<()> {
    let scan = run_scan(command.config_path, command.paths, command.include_global)?;
    let plan = build_plan(&scan);
    write_or_print_plan(&plan, command.output)
}

pub fn dry_run(command: DryRunCommand) -> Result<()> {
    let scan = run_scan(command.config_path, command.paths, command.include_global)?;
    let plan = build_plan(&scan);
    if let Some(path) = command.output {
        write_cleanup_plan(&plan, &path)?;
        println!("Dry run wrote {}", path.display());
    }
    if command.json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
    } else {
        println!(
            "Dry run: {} candidate(s), {} selected, {} selected bytes. No files were changed.",
            plan.summary.candidate_count,
            plan.summary.selected_count,
            format_bytes(plan.summary.selected_size_bytes)
        );
    }
    Ok(())
}

pub fn restore_list() -> Result<()> {
    let repository = ManifestRepository::new(default_state_dir());
    let (runs, restores) = repository.history()?;
    let restored = restored_run_ids(&restores);
    if runs.is_empty() {
        println!("No cleanup runs found");
        return Ok(());
    }
    for run in runs {
        let state = if restored.contains(run.run_id.as_str()) {
            "restored"
        } else {
            "available"
        };
        println!(
            "{} {} succeeded={} failed={} {}",
            run.run_id,
            run.created_at.to_rfc3339(),
            run.summary.succeeded,
            run.summary.failed,
            state
        );
    }
    Ok(())
}

pub fn restore_run(run_id: &str, confirm: bool) -> Result<()> {
    if !confirm {
        bail!("restore run requires --confirm");
    }
    let repository = ManifestRepository::new(default_state_dir());
    let manifest = repository
        .find_execution(run_id)?
        .with_context(|| format!("cleanup run {run_id} was not found"))?;
    let restore =
        restore_execution_manifest(&manifest, &SystemRestoreExecutor, repository.state_dir())?;
    println!(
        "Restored {} item(s), failed {} item(s), restore id {}.",
        restore.summary.succeeded, restore.summary.failed, restore.restore_id
    );
    Ok(())
}

fn run_scan(
    config_path: Option<PathBuf>,
    paths: Vec<PathBuf>,
    include_global: bool,
) -> Result<WorkflowScan> {
    let config_path_for_policy = config_path.clone().or_else(default_config_path);
    let config = load_config(config_path)?;
    let registry = RuleRegistry::load(&config)?;
    let roots = resolve_roots(paths, include_global)?;
    let options = ScanOptions {
        stay_on_filesystem: config.scan.stay_on_filesystem,
        ignore_dirs: config.scan.ignore_dirs.clone(),
        ignore_patterns: config.scan.ignore_patterns.clone(),
    };
    let mut report = scan_paths(&roots, &options)?;
    registry.annotate_entries(&mut report.entries);
    let roots = report.summary.roots.clone();
    Ok(WorkflowScan {
        config,
        config_path: config_path_for_policy,
        registry,
        roots,
        report,
    })
}

fn build_plan(scan: &WorkflowScan) -> CleanupPlan {
    build_cleanup_plan_with_policy(
        scan.roots.clone(),
        scan.registry.versions(),
        &scan.report.entries,
        &safety_policy(&scan.config, scan.config_path.clone()),
    )
}

fn write_or_print_plan(plan: &CleanupPlan, output: Option<PathBuf>) -> Result<()> {
    if let Some(path) = output {
        write_cleanup_plan(plan, &path)?;
        println!("Wrote {}", path.display());
    } else {
        println!("{}", serde_json::to_string_pretty(plan)?);
    }
    Ok(())
}

fn print_scan_json(report: &ScanReport) -> Result<()> {
    let errors = report
        .errors
        .iter()
        .map(|error| {
            json!({
                "path": error.path.as_ref().map(|path| path.display().to_string()),
                "message": error.message,
            })
        })
        .collect::<Vec<_>>();
    let value = json!({
        "summary": report.summary,
        "entries": report.entries,
        "errors": errors,
    });
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}

fn load_config(path: Option<PathBuf>) -> Result<Config> {
    match path {
        Some(path) => Config::load_from(path),
        None => Config::load(),
    }
}

fn resolve_roots(mut paths: Vec<PathBuf>, include_global: bool) -> Result<Vec<PathBuf>> {
    if paths.is_empty() {
        paths.push(std::env::current_dir()?);
    }
    if include_global {
        paths.extend(developer_cache_roots());
    }
    paths.sort();
    paths.dedup();
    if paths.is_empty() {
        bail!("no scan roots were provided and no global cache roots were found");
    }
    Ok(paths)
}

fn safety_policy(config: &Config, config_path: Option<PathBuf>) -> SafetyPolicy {
    let mut protected = Vec::new();
    protected.extend(cleanr_config::home_dir());
    protected.extend(config_path);
    if let Ok(executable) = std::env::current_exe() {
        protected.push(executable);
    }
    let mut protected_subtrees = vec![default_state_dir()];
    protected_subtrees.extend(config.plugins.dirs.iter().cloned());
    protected_subtrees.extend(config.i18n.dirs.iter().cloned());
    SafetyPolicy::new(protected, config.cleanup.require_confirm)
        .with_protected_subtrees(protected_subtrees)
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KiB", "MiB", "GiB", "TiB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}
