#![forbid(unsafe_code)]

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const CLEANUP_PLAN_SCHEMA_VERSION: &str = "cleanr.cleanup-plan.v1";
pub const EXECUTION_SCHEMA_VERSION: &str = "cleanr.execution.v1";
pub const RESTORE_SCHEMA_VERSION: &str = "cleanr.restore.v1";

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
#[serde(rename_all = "kebab-case")]
pub enum Confidence {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
    Other,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum PlannedAction {
    Trash,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "kebab-case")]
pub enum RuleTrust {
    #[default]
    Untrusted,
    Trusted,
    Builtin,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuleHit {
    pub rule_pack_id: String,
    pub rule_id: String,
    pub label: String,
    pub category: String,
    pub confidence: Confidence,
    pub reason: String,
    pub risk_note: String,
    pub default_selected: bool,
    #[serde(default)]
    pub trust: RuleTrust,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScanEntry {
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size_bytes: u64,
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub rule_hits: Vec<RuleHit>,
}

impl ScanEntry {
    #[must_use]
    pub fn file_name(&self) -> Option<String> {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ScanSummary {
    pub roots: Vec<PathBuf>,
    pub entries_seen: usize,
    pub errors: usize,
    pub total_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupPlan {
    pub schema_version: String,
    pub created_at: DateTime<Utc>,
    pub scan_roots: Vec<PathBuf>,
    pub ruleset_versions: Vec<RulesetVersion>,
    pub summary: PlanSummary,
    pub items: Vec<CleanupItem>,
    pub safety: PlanSafety,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RulesetVersion {
    pub id: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct PlanSummary {
    pub candidate_count: usize,
    pub selected_count: usize,
    pub selected_size_bytes: u64,
    pub total_candidate_size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlanSafety {
    pub default_action: PlannedAction,
    pub requires_confirmation: bool,
    pub agent_can_execute: bool,
    pub rollback_method: String,
    #[serde(default)]
    pub protected_paths: Vec<PathBuf>,
    #[serde(default)]
    pub protected_subtrees: Vec<PathBuf>,
}

impl Default for PlanSafety {
    fn default() -> Self {
        Self {
            default_action: PlannedAction::Trash,
            requires_confirmation: true,
            agent_can_execute: false,
            rollback_method: "system-trash+manifest".to_string(),
            protected_paths: Vec::new(),
            protected_subtrees: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupItem {
    pub path: PathBuf,
    pub kind: EntryKind,
    pub size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tree_fingerprint: Option<CleanupItemFingerprint>,
    pub rule_id: String,
    pub category: String,
    pub confidence: Confidence,
    pub reason: String,
    pub risk_note: String,
    pub selected: bool,
    pub planned_action: PlannedAction,
    pub rollback_method: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CleanupItemFingerprint {
    pub descendants: usize,
    pub total_size_bytes: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_modified_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionManifest {
    pub schema_version: String,
    pub run_id: String,
    pub created_at: DateTime<Utc>,
    pub plan_schema_version: String,
    pub summary: ExecutionSummary,
    pub items: Vec<ExecutionItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ExecutionSummary {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionItem {
    pub path: PathBuf,
    pub planned_action: PlannedAction,
    pub status: ExecutionStatus,
    pub rule_id: String,
    pub rollback_receipt: Option<RollbackReceipt>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ExecutionStatus {
    Pending,
    Trashed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RollbackReceipt {
    pub method: String,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub locator: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreManifest {
    pub schema_version: String,
    pub restore_id: String,
    pub source_run_id: String,
    pub created_at: DateTime<Utc>,
    pub summary: RestoreSummary,
    pub items: Vec<RestoreItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RestoreSummary {
    pub attempted: usize,
    pub succeeded: usize,
    pub failed: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RestoreItem {
    pub path: PathBuf,
    pub status: RestoreStatus,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RestoreStatus {
    Restored,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SafetyPolicy {
    protected_paths: Vec<PathBuf>,
    protected_subtrees: Vec<PathBuf>,
    requires_confirmation: bool,
}

impl SafetyPolicy {
    #[must_use]
    pub fn new(protected_paths: Vec<PathBuf>, requires_confirmation: bool) -> Self {
        Self {
            protected_paths: normalize_protected_paths(protected_paths),
            protected_subtrees: Vec::new(),
            requires_confirmation,
        }
    }

    #[must_use]
    pub fn with_protected_subtrees(mut self, protected_subtrees: Vec<PathBuf>) -> Self {
        self.protected_subtrees = normalize_protected_paths(protected_subtrees);
        self
    }

    #[must_use]
    pub fn protected_paths(&self) -> &[PathBuf] {
        &self.protected_paths
    }

    #[must_use]
    pub fn protected_subtrees(&self) -> &[PathBuf] {
        &self.protected_subtrees
    }

    #[must_use]
    pub fn allows_candidate(&self, path: &std::path::Path) -> bool {
        let normalized_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        !is_filesystem_root(&normalized_path)
            && !self
                .protected_paths
                .iter()
                .any(|protected| protected.starts_with(&normalized_path))
            && !self.protected_subtrees.iter().any(|protected| {
                protected.starts_with(&normalized_path) || normalized_path.starts_with(protected)
            })
    }
}

#[must_use]
pub fn build_cleanup_plan(
    scan_roots: Vec<PathBuf>,
    ruleset_versions: Vec<RulesetVersion>,
    entries: &[ScanEntry],
) -> CleanupPlan {
    build_cleanup_plan_with_policy(
        scan_roots,
        ruleset_versions,
        entries,
        &SafetyPolicy::new(Vec::new(), true),
    )
}

#[must_use]
pub fn build_cleanup_plan_with_policy(
    scan_roots: Vec<PathBuf>,
    ruleset_versions: Vec<RulesetVersion>,
    entries: &[ScanEntry],
    policy: &SafetyPolicy,
) -> CleanupPlan {
    let normalized_scan_roots = normalize_protected_paths(scan_roots.clone());
    let tree_fingerprints = tree_fingerprints(entries);
    let items = entries
        .iter()
        .filter_map(|entry| {
            let hit = best_hit(entry)?;
            let normalized_path = entry
                .path
                .canonicalize()
                .unwrap_or_else(|_| entry.path.clone());
            if normalized_scan_roots
                .iter()
                .any(|root| root == &normalized_path)
                || !policy.allows_candidate(&entry.path)
            {
                return None;
            }
            let selected = hit.default_selected
                && hit.confidence == Confidence::High
                && hit.trust != RuleTrust::Untrusted;
            Some(CleanupItem {
                path: entry.path.clone(),
                kind: entry.kind,
                size_bytes: entry.size_bytes,
                modified_at: entry.modified_at,
                tree_fingerprint: (entry.kind == EntryKind::Directory)
                    .then(|| tree_fingerprints.get(&entry.path).cloned())
                    .flatten(),
                rule_id: format!("{}:{}", hit.rule_pack_id, hit.rule_id),
                category: hit.category.clone(),
                confidence: hit.confidence,
                reason: hit.reason.clone(),
                risk_note: hit.risk_note.clone(),
                selected,
                planned_action: PlannedAction::Trash,
                rollback_method: "system-trash+manifest".to_string(),
            })
        })
        .collect::<Vec<_>>();

    let mut items = remove_overlapping_items(items);
    items.sort_by(|a, b| {
        b.selected
            .cmp(&a.selected)
            .then_with(|| b.confidence.cmp(&a.confidence))
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
            .then_with(|| a.path.cmp(&b.path))
    });

    let summary = PlanSummary {
        candidate_count: items.len(),
        selected_count: items.iter().filter(|item| item.selected).count(),
        selected_size_bytes: items
            .iter()
            .filter(|item| item.selected)
            .map(|item| item.size_bytes)
            .sum(),
        total_candidate_size_bytes: items.iter().map(|item| item.size_bytes).sum(),
    };

    CleanupPlan {
        schema_version: CLEANUP_PLAN_SCHEMA_VERSION.to_string(),
        created_at: Utc::now(),
        scan_roots,
        ruleset_versions,
        summary,
        items,
        safety: PlanSafety {
            requires_confirmation: policy.requires_confirmation,
            protected_paths: policy.protected_paths.clone(),
            protected_subtrees: policy.protected_subtrees.clone(),
            ..PlanSafety::default()
        },
    }
}

fn remove_overlapping_items(mut items: Vec<CleanupItem>) -> Vec<CleanupItem> {
    items.sort_by(|a, b| {
        b.selected
            .cmp(&a.selected)
            .then_with(|| b.confidence.cmp(&a.confidence))
            .then_with(|| b.size_bytes.cmp(&a.size_bytes))
            .then_with(|| {
                a.path
                    .components()
                    .count()
                    .cmp(&b.path.components().count())
            })
            .then_with(|| a.path.cmp(&b.path))
    });

    let mut non_overlapping: Vec<CleanupItem> = Vec::with_capacity(items.len());
    let mut kept_paths = HashSet::with_capacity(items.len());
    let mut kept_ancestor_paths = HashSet::with_capacity(items.len());
    for item in items {
        if overlaps_kept_path(&item.path, &kept_paths, &kept_ancestor_paths) {
            continue;
        }
        remember_kept_path(&item.path, &mut kept_paths, &mut kept_ancestor_paths);
        non_overlapping.push(item);
    }
    non_overlapping
}

fn overlaps_kept_path(
    path: &Path,
    kept_paths: &HashSet<PathBuf>,
    kept_ancestor_paths: &HashSet<PathBuf>,
) -> bool {
    path.ancestors()
        .filter(|ancestor| !ancestor.as_os_str().is_empty())
        .any(|ancestor| kept_paths.contains(ancestor))
        || kept_ancestor_paths.contains(path)
}

fn remember_kept_path(
    path: &Path,
    kept_paths: &mut HashSet<PathBuf>,
    kept_ancestor_paths: &mut HashSet<PathBuf>,
) {
    kept_paths.insert(path.to_path_buf());
    kept_ancestor_paths.extend(
        path.ancestors()
            .filter(|ancestor| !ancestor.as_os_str().is_empty())
            .map(Path::to_path_buf),
    );
}

fn tree_fingerprints(entries: &[ScanEntry]) -> HashMap<PathBuf, CleanupItemFingerprint> {
    let mut fingerprints = entries
        .iter()
        .filter(|entry| entry.kind == EntryKind::Directory)
        .map(|entry| {
            (
                entry.path.clone(),
                CleanupItemFingerprint {
                    descendants: 0,
                    total_size_bytes: entry.size_bytes,
                    latest_modified_at: entry.modified_at,
                },
            )
        })
        .collect::<HashMap<_, _>>();

    let mut indices = (0..entries.len()).collect::<Vec<_>>();
    indices.sort_by_key(|idx| std::cmp::Reverse(entries[*idx].path.components().count()));

    for idx in indices {
        let entry = &entries[idx];
        let Some(parent) = entry.path.parent() else {
            continue;
        };
        let child_fingerprint = (entry.kind == EntryKind::Directory)
            .then(|| fingerprints.get(&entry.path).cloned())
            .flatten();
        let descendants = 1 + child_fingerprint
            .as_ref()
            .map_or(0, |fingerprint| fingerprint.descendants);
        let latest_modified_at = child_fingerprint
            .and_then(|fingerprint| fingerprint.latest_modified_at)
            .or(entry.modified_at);
        if let Some(parent_fingerprint) = fingerprints.get_mut(parent) {
            parent_fingerprint.descendants += descendants;
            parent_fingerprint.latest_modified_at =
                max_datetime(parent_fingerprint.latest_modified_at, latest_modified_at);
        }
    }

    fingerprints
}

fn max_datetime(
    left: Option<DateTime<Utc>>,
    right: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn best_hit(entry: &ScanEntry) -> Option<&RuleHit> {
    entry.rule_hits.iter().max_by(|a, b| {
        a.trust
            .cmp(&b.trust)
            .then_with(|| a.default_selected.cmp(&b.default_selected))
            .then_with(|| a.confidence.cmp(&b.confidence))
    })
}

fn normalize_protected_paths(mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    for path in &mut paths {
        if let Ok(canonical) = path.canonicalize() {
            *path = canonical;
        }
    }
    paths.sort();
    paths.dedup();
    paths
}

fn is_filesystem_root(path: &std::path::Path) -> bool {
    path.is_absolute() && path.parent().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_selects_only_high_confidence_defaults() {
        let entries = vec![
            ScanEntry {
                path: PathBuf::from("node_modules"),
                kind: EntryKind::Directory,
                size_bytes: 10,
                modified_at: None,
                rule_hits: vec![RuleHit {
                    rule_pack_id: "builtin-dev".into(),
                    rule_id: "node-modules".into(),
                    label: "Node modules".into(),
                    category: "developer-cache".into(),
                    confidence: Confidence::High,
                    reason: "reinstallable dependency cache".into(),
                    risk_note: "can be restored by package manager".into(),
                    default_selected: true,
                    trust: RuleTrust::Builtin,
                }],
            },
            ScanEntry {
                path: PathBuf::from("Downloads/big.zip"),
                kind: EntryKind::File,
                size_bytes: 20,
                modified_at: None,
                rule_hits: vec![RuleHit {
                    rule_pack_id: "builtin-general".into(),
                    rule_id: "large-download".into(),
                    label: "Large download".into(),
                    category: "downloads".into(),
                    confidence: Confidence::Low,
                    reason: "large download".into(),
                    risk_note: "user data review required".into(),
                    default_selected: false,
                    trust: RuleTrust::Builtin,
                }],
            },
        ];

        let plan = build_cleanup_plan(vec![PathBuf::from(".")], vec![], &entries);
        assert_eq!(plan.summary.candidate_count, 2);
        assert_eq!(plan.summary.selected_count, 1);
        assert!(plan.items.iter().any(|item| item.selected));
        assert!(!plan.safety.agent_can_execute);
    }

    #[test]
    fn plan_serializes_as_json_manifest() {
        let plan = build_cleanup_plan(vec![PathBuf::from(".")], vec![], &[]);
        let json = serde_json::to_string(&plan).expect("plan serializes");
        assert!(json.contains(CLEANUP_PLAN_SCHEMA_VERSION));
        assert!(json.contains("system-trash+manifest"));
    }

    #[test]
    fn plan_removes_overlapping_parent_and_child_candidates() {
        let hit = |rule_id: &str| RuleHit {
            rule_pack_id: "builtin-dev".into(),
            rule_id: rule_id.into(),
            label: rule_id.into(),
            category: "developer-cache".into(),
            confidence: Confidence::High,
            reason: "generated".into(),
            risk_note: "rebuild".into(),
            default_selected: true,
            trust: RuleTrust::Builtin,
        };
        let entries = vec![
            ScanEntry {
                path: PathBuf::from("/repo/node_modules"),
                kind: EntryKind::Directory,
                size_bytes: 100,
                modified_at: None,
                rule_hits: vec![hit("node-modules")],
            },
            ScanEntry {
                path: PathBuf::from("/repo/node_modules/pkg/node_modules"),
                kind: EntryKind::Directory,
                size_bytes: 40,
                modified_at: None,
                rule_hits: vec![hit("node-modules")],
            },
        ];

        let plan = build_cleanup_plan(vec![PathBuf::from("/repo")], vec![], &entries);
        assert_eq!(plan.summary.candidate_count, 1);
        assert_eq!(plan.summary.selected_size_bytes, 100);
        assert_eq!(plan.items[0].path, PathBuf::from("/repo/node_modules"));
    }

    #[test]
    fn directory_candidates_include_tree_fingerprints() {
        let hit = RuleHit {
            rule_pack_id: "builtin-dev".into(),
            rule_id: "cache".into(),
            label: "Cache".into(),
            category: "developer-cache".into(),
            confidence: Confidence::High,
            reason: "generated".into(),
            risk_note: "rebuild".into(),
            default_selected: true,
            trust: RuleTrust::Builtin,
        };
        let modified_at = Utc::now();
        let newer_modified_at = modified_at + chrono::Duration::seconds(1);
        let entries = vec![
            ScanEntry {
                path: PathBuf::from("/repo/cache"),
                kind: EntryKind::Directory,
                size_bytes: 7,
                modified_at: Some(modified_at),
                rule_hits: vec![hit],
            },
            ScanEntry {
                path: PathBuf::from("/repo/cache/file"),
                kind: EntryKind::File,
                size_bytes: 3,
                modified_at: Some(modified_at),
                rule_hits: Vec::new(),
            },
            ScanEntry {
                path: PathBuf::from("/repo/cache/nested"),
                kind: EntryKind::Directory,
                size_bytes: 4,
                modified_at: Some(modified_at),
                rule_hits: Vec::new(),
            },
            ScanEntry {
                path: PathBuf::from("/repo/cache/nested/file"),
                kind: EntryKind::File,
                size_bytes: 4,
                modified_at: Some(newer_modified_at),
                rule_hits: Vec::new(),
            },
        ];

        let plan = build_cleanup_plan(vec![PathBuf::from("/repo")], vec![], &entries);

        assert_eq!(
            plan.items[0].tree_fingerprint,
            Some(CleanupItemFingerprint {
                descendants: 3,
                total_size_bytes: 7,
                latest_modified_at: Some(newer_modified_at),
            })
        );
    }

    #[test]
    fn selected_child_wins_over_unselected_parent() {
        let entries = vec![
            ScanEntry {
                path: PathBuf::from("/repo/cache"),
                kind: EntryKind::Directory,
                size_bytes: 100,
                modified_at: None,
                rule_hits: vec![RuleHit {
                    rule_pack_id: "custom".into(),
                    rule_id: "review-parent".into(),
                    label: "review".into(),
                    category: "cache".into(),
                    confidence: Confidence::Medium,
                    reason: "review".into(),
                    risk_note: "review".into(),
                    default_selected: false,
                    trust: RuleTrust::Trusted,
                }],
            },
            ScanEntry {
                path: PathBuf::from("/repo/cache/generated"),
                kind: EntryKind::Directory,
                size_bytes: 40,
                modified_at: None,
                rule_hits: vec![RuleHit {
                    rule_pack_id: "custom".into(),
                    rule_id: "safe-child".into(),
                    label: "safe".into(),
                    category: "cache".into(),
                    confidence: Confidence::High,
                    reason: "generated".into(),
                    risk_note: "rebuild".into(),
                    default_selected: true,
                    trust: RuleTrust::Trusted,
                }],
            },
        ];

        let plan = build_cleanup_plan(vec![PathBuf::from("/repo")], vec![], &entries);
        assert_eq!(plan.summary.candidate_count, 1);
        assert_eq!(plan.items[0].path, PathBuf::from("/repo/cache/generated"));
        assert!(plan.items[0].selected);
    }

    #[test]
    fn protected_subtrees_reject_their_children_and_parents() {
        let policy = SafetyPolicy::new(vec![], true)
            .with_protected_subtrees(vec![PathBuf::from("/repo/.cleanr")]);

        assert!(!policy.allows_candidate(std::path::Path::new("/repo")));
        assert!(!policy.allows_candidate(std::path::Path::new("/repo/.cleanr/history")));
        assert!(policy.allows_candidate(std::path::Path::new("/repo/target")));
    }

    #[test]
    fn scan_root_itself_is_never_a_cleanup_candidate() {
        let entry = ScanEntry {
            path: PathBuf::from("/repo"),
            kind: EntryKind::Directory,
            size_bytes: 100,
            modified_at: None,
            rule_hits: vec![RuleHit {
                rule_pack_id: "trusted".into(),
                rule_id: "broad".into(),
                label: "Broad".into(),
                category: "cache".into(),
                confidence: Confidence::High,
                reason: "generated".into(),
                risk_note: "dangerous".into(),
                default_selected: true,
                trust: RuleTrust::Trusted,
            }],
        };

        let plan = build_cleanup_plan(vec![PathBuf::from("/repo")], vec![], &[entry]);

        assert!(plan.items.is_empty());
    }

    #[test]
    fn best_rule_hit_prefers_trust_before_default_selection_and_confidence() {
        let hit = |rule_id: &str,
                   trust: RuleTrust,
                   confidence: Confidence,
                   default_selected: bool| RuleHit {
            rule_pack_id: "pack".into(),
            rule_id: rule_id.into(),
            label: rule_id.into(),
            category: "cache".into(),
            confidence,
            reason: rule_id.into(),
            risk_note: "review".into(),
            default_selected,
            trust,
        };
        let entry = ScanEntry {
            path: PathBuf::from("/repo/cache"),
            kind: EntryKind::Directory,
            size_bytes: 42,
            modified_at: None,
            rule_hits: vec![
                hit("untrusted", RuleTrust::Untrusted, Confidence::High, true),
                hit("trusted", RuleTrust::Trusted, Confidence::Medium, false),
                hit("builtin", RuleTrust::Builtin, Confidence::Low, false),
            ],
        };

        let plan = build_cleanup_plan(vec![PathBuf::from("/repo")], vec![], &[entry]);

        assert_eq!(plan.items[0].rule_id, "pack:builtin");
        assert!(!plan.items[0].selected);
    }

    #[test]
    fn protected_paths_are_normalized_sorted_and_deduplicated() {
        let temp = tempfile::tempdir().expect("tempdir");
        let protected = temp.path().join("protected");
        std::fs::create_dir(&protected).expect("protected dir");
        let policy = SafetyPolicy::new(
            vec![
                protected.clone(),
                temp.path().join(".").join("protected"),
                temp.path().join("another"),
            ],
            false,
        );

        assert_eq!(policy.protected_paths().len(), 2);
        assert!(!policy.allows_candidate(&protected));
        assert!(!policy.allows_candidate(temp.path()));
        assert!(!policy.requires_confirmation);
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_root_is_never_allowed() {
        assert!(!SafetyPolicy::default().allows_candidate(std::path::Path::new("/")));
    }
}
