#![forbid(unsafe_code)]

use std::{
    collections::{HashMap, HashSet},
    fs::Metadata,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};
use cleanr_core::{EntryKind, ScanEntry, ScanSummary};
use globset::{Glob, GlobSet, GlobSetBuilder};
use walkdir::WalkDir;

pub const SCAN_CANCELLED: &str = "scan cancelled";

#[derive(Debug, Clone, Default)]
pub struct ScanOptions {
    pub stay_on_filesystem: bool,
    pub ignore_dirs: Vec<PathBuf>,
    pub ignore_patterns: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct ScanReport {
    pub summary: ScanSummary,
    pub entries: Vec<ScanEntry>,
    pub errors: Vec<ScanError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanPhase {
    Discovering,
    Scanning,
    Aggregating,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanProgress {
    pub phase: ScanPhase,
    pub entries_total: usize,
    pub entries_scanned: usize,
    pub bytes_scanned: u64,
    pub errors: usize,
    pub current_path: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanError {
    pub path: Option<PathBuf>,
    pub message: String,
}

pub fn scan_paths(paths: &[PathBuf], options: &ScanOptions) -> Result<ScanReport> {
    scan_paths_impl(paths, options, None, &mut |_| {})
}

pub fn scan_paths_with_progress(
    paths: &[PathBuf],
    options: &ScanOptions,
    mut on_progress: impl FnMut(ScanProgress),
) -> Result<ScanReport> {
    scan_paths_impl(paths, options, None, &mut on_progress)
}

pub fn scan_paths_with_progress_cancellable(
    paths: &[PathBuf],
    options: &ScanOptions,
    cancelled: &AtomicBool,
    mut on_progress: impl FnMut(ScanProgress),
) -> Result<ScanReport> {
    scan_paths_impl(paths, options, Some(cancelled), &mut on_progress)
}

fn scan_paths_impl(
    paths: &[PathBuf],
    options: &ScanOptions,
    cancelled: Option<&AtomicBool>,
    on_progress: &mut dyn FnMut(ScanProgress),
) -> Result<ScanReport> {
    let roots = normalize_roots(if paths.is_empty() {
        vec![std::env::current_dir()?]
    } else {
        paths.to_vec()
    });
    let ignore = IgnoreMatcher::new(options)?;

    let mut report = ScanReport {
        summary: ScanSummary {
            roots: roots.clone(),
            ..ScanSummary::default()
        },
        ..ScanReport::default()
    };

    let mut hardlinks = HardlinkTracker::default();
    let mut progress = ScanProgressTracker::new(on_progress);
    for root in &roots {
        let result = scan_root(
            root,
            options,
            &ignore,
            cancelled,
            &mut hardlinks,
            &mut report,
            &mut progress,
        );
        if let Err(err) = result {
            if err.to_string() == SCAN_CANCELLED {
                return Err(err);
            }
            return Err(err).with_context(|| format!("failed to scan {}", root.display()));
        }
    }

    (progress.on_progress)(ScanProgress {
        phase: ScanPhase::Aggregating,
        entries_total: progress.entries_scanned,
        entries_scanned: progress.entries_scanned,
        bytes_scanned: progress.bytes_scanned,
        errors: report.errors.len(),
        current_path: None,
    });
    aggregate_directory_sizes(&mut report.entries);
    report.summary.entries_seen = report.entries.len();
    report.summary.errors = report.errors.len();
    report.summary.total_size_bytes = report
        .entries
        .iter()
        .filter(|entry| report.summary.roots.iter().any(|root| &entry.path == root))
        .map(|entry| entry.size_bytes)
        .sum();

    Ok(report)
}

fn scan_root(
    root: &Path,
    options: &ScanOptions,
    ignore: &IgnoreMatcher,
    cancelled: Option<&AtomicBool>,
    hardlinks: &mut HardlinkTracker,
    report: &mut ScanReport,
    progress: &mut ScanProgressTracker<'_>,
) -> Result<()> {
    let root_device = root_device(root, options);

    let mut walker = WalkDir::new(root).follow_links(false).into_iter();
    while let Some(next) = walker.next() {
        if cancelled.is_some_and(|flag| flag.load(Ordering::Relaxed)) {
            bail!(SCAN_CANCELLED);
        }
        let entry = match next {
            Ok(entry) => entry,
            Err(err) => {
                report.errors.push(ScanError {
                    path: err.path().map(Path::to_path_buf),
                    message: err.to_string(),
                });
                continue;
            }
        };

        let path = entry.path().to_path_buf();
        let metadata = match entry.path().symlink_metadata() {
            Ok(metadata) => metadata,
            Err(err) => {
                report.errors.push(ScanError {
                    path: Some(path),
                    message: err.to_string(),
                });
                continue;
            }
        };

        if entry.file_type().is_dir() {
            if ignore.matches(&path, root) {
                walker.skip_current_dir();
                continue;
            }
            if let Some(root_device) = root_device
                && device_id(&metadata).is_some_and(|device| device != root_device)
            {
                walker.skip_current_dir();
                continue;
            }
        } else if ignore.matches(&path, root) {
            continue;
        }

        let kind = kind_of(&metadata);
        let size_bytes = if kind == EntryKind::File && hardlinks.insert(&metadata) {
            metadata.len()
        } else {
            0
        };

        progress.record(&path, size_bytes, report.errors.len());

        report.entries.push(ScanEntry {
            path,
            kind,
            size_bytes,
            modified_at: metadata.modified().ok().map(DateTime::<Utc>::from),
            rule_hits: vec![],
        });
    }

    Ok(())
}

struct ScanProgressTracker<'a> {
    entries_scanned: usize,
    bytes_scanned: u64,
    on_progress: &'a mut dyn FnMut(ScanProgress),
}

impl<'a> ScanProgressTracker<'a> {
    fn new(on_progress: &'a mut dyn FnMut(ScanProgress)) -> Self {
        Self {
            entries_scanned: 0,
            bytes_scanned: 0,
            on_progress,
        }
    }

    fn record(&mut self, path: &Path, size_bytes: u64, errors: usize) {
        self.entries_scanned += 1;
        self.bytes_scanned = self.bytes_scanned.saturating_add(size_bytes);
        if should_emit_progress(self.entries_scanned) {
            (self.on_progress)(ScanProgress {
                phase: ScanPhase::Scanning,
                entries_total: 0,
                entries_scanned: self.entries_scanned,
                bytes_scanned: self.bytes_scanned,
                errors,
                current_path: Some(path.to_path_buf()),
            });
        }
    }
}

fn root_device(root: &Path, options: &ScanOptions) -> Option<u64> {
    if options.stay_on_filesystem {
        root.symlink_metadata()
            .ok()
            .and_then(|metadata| device_id(&metadata))
    } else {
        None
    }
}

fn should_emit_progress(entries: usize) -> bool {
    entries <= 16 || entries.is_multiple_of(64)
}

fn aggregate_directory_sizes(entries: &mut [ScanEntry]) {
    let by_path = entries
        .iter()
        .enumerate()
        .map(|(idx, entry)| (entry.path.clone(), idx))
        .collect::<HashMap<_, _>>();

    let mut indices = (0..entries.len()).collect::<Vec<_>>();
    indices.sort_by_key(|idx| std::cmp::Reverse(entries[*idx].path.components().count()));

    for idx in indices {
        let size = entries[idx].size_bytes;
        let Some(parent) = entries[idx].path.parent() else {
            continue;
        };
        if let Some(parent_idx) = by_path.get(parent) {
            entries[*parent_idx].size_bytes = entries[*parent_idx].size_bytes.saturating_add(size);
        }
    }
}

fn kind_of(metadata: &Metadata) -> EntryKind {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        EntryKind::Symlink
    } else if file_type.is_dir() {
        EntryKind::Directory
    } else if file_type.is_file() {
        EntryKind::File
    } else {
        EntryKind::Other
    }
}

struct IgnoreMatcher {
    dirs: Vec<PathBuf>,
    patterns: GlobSet,
}

impl IgnoreMatcher {
    fn new(options: &ScanOptions) -> Result<Self> {
        let mut builder = GlobSetBuilder::new();
        for pattern in &options.ignore_patterns {
            builder.add(
                Glob::new(pattern)
                    .with_context(|| format!("invalid scan ignore pattern: {pattern}"))?,
            );
        }
        Ok(Self {
            dirs: options
                .ignore_dirs
                .iter()
                .map(|path| path.canonicalize().unwrap_or_else(|_| path.clone()))
                .collect(),
            patterns: builder.build()?,
        })
    }

    fn matches(&self, path: &Path, root: &Path) -> bool {
        if self
            .dirs
            .iter()
            .any(|ignored| path == ignored || path.starts_with(ignored))
        {
            return true;
        }
        let absolute = normalized_path(path);
        let relative = path
            .strip_prefix(root)
            .map(normalized_path)
            .unwrap_or_default();
        self.patterns.is_match(&absolute) || self.patterns.is_match(&relative)
    }
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn normalize_roots(mut roots: Vec<PathBuf>) -> Vec<PathBuf> {
    for root in &mut roots {
        if let Ok(canonical) = root.canonicalize() {
            *root = canonical;
        }
    }
    roots.sort_by(|a, b| {
        a.components()
            .count()
            .cmp(&b.components().count())
            .then_with(|| a.cmp(b))
    });
    let mut normalized = Vec::<PathBuf>::new();
    for root in roots {
        if normalized.iter().any(|parent| root.starts_with(parent)) {
            continue;
        }
        normalized.push(root);
    }
    normalized
}

#[must_use]
pub fn developer_cache_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = dirs::home_dir() {
        roots.extend([
            home.join(".cargo").join("registry"),
            home.join(".cargo").join("git"),
            home.join(".npm"),
            home.join(".cache").join("pnpm"),
            home.join(".cache").join("yarn"),
            home.join(".cache").join("pip"),
            home.join(".cache").join("uv"),
            home.join(".local").join("share").join("pnpm").join("store"),
            home.join(".gradle").join("caches"),
            home.join(".m2").join("repository"),
            home.join("go").join("pkg").join("mod"),
        ]);
        #[cfg(target_os = "macos")]
        roots.extend([
            home.join("Library").join("Caches").join("pip"),
            home.join("Library").join("Caches").join("uv"),
            home.join("Library").join("Caches").join("Yarn"),
            home.join("Library").join("pnpm").join("store"),
            home.join("Library")
                .join("Developer")
                .join("Xcode")
                .join("DerivedData"),
        ]);
    }
    if let Some(cache) = dirs::cache_dir() {
        roots.extend([
            cache.join("npm"),
            cache.join("pnpm"),
            cache.join("yarn"),
            cache.join("pip"),
            cache.join("uv"),
        ]);
    }
    roots.retain(|path| path.exists());
    normalize_roots(roots)
}

#[derive(Default)]
struct HardlinkTracker {
    seen: HashSet<FileIdentity>,
}

impl HardlinkTracker {
    fn insert(&mut self, metadata: &Metadata) -> bool {
        let Some(identity) = file_identity(metadata) else {
            return true;
        };
        self.seen.insert(identity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct FileIdentity {
    device: u64,
    inode: u64,
}

#[cfg(unix)]
fn file_identity(metadata: &Metadata) -> Option<FileIdentity> {
    use std::os::unix::fs::MetadataExt;

    (metadata.nlink() > 1).then_some(FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
    })
}

#[cfg(not(unix))]
fn file_identity(_metadata: &Metadata) -> Option<FileIdentity> {
    None
}

#[cfg(unix)]
fn device_id(metadata: &Metadata) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    Some(metadata.dev())
}

#[cfg(not(unix))]
fn device_id(_metadata: &Metadata) -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn scan_does_not_follow_symlinks() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        fs::create_dir(root.join("cache")).expect("mkdir");
        fs::write(root.join("cache").join("file"), b"1234").expect("write");

        #[cfg(unix)]
        std::os::unix::fs::symlink(root.join("cache"), root.join("cache-link")).expect("symlink");

        let report = scan_paths(&[root.to_path_buf()], &ScanOptions::default()).expect("scan");
        let link = report
            .entries
            .iter()
            .find(|entry| entry.path.ends_with("cache-link"));

        #[cfg(unix)]
        assert_eq!(link.map(|entry| entry.kind), Some(EntryKind::Symlink));
        assert!(
            !report
                .entries
                .iter()
                .any(|entry| entry.path.ends_with("cache-link/file"))
        );
    }

    #[test]
    fn directory_sizes_include_children() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("target")).expect("mkdir");
        fs::write(temp.path().join("target").join("artifact"), vec![0; 12]).expect("write");

        let report =
            scan_paths(&[temp.path().to_path_buf()], &ScanOptions::default()).expect("scan");
        let target = report
            .entries
            .iter()
            .find(|entry| entry.path.ends_with("target"))
            .expect("target entry");

        assert_eq!(target.size_bytes, 12);
    }

    #[test]
    fn progress_scans_each_entry_once_and_finishes_with_total() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join("cache")).expect("mkdir");
        fs::write(temp.path().join("cache").join("one"), b"1234").expect("write");
        fs::write(temp.path().join("two"), b"12").expect("write");

        let mut progress = Vec::new();
        let report = scan_paths_with_progress(
            &[temp.path().to_path_buf()],
            &ScanOptions::default(),
            |event| progress.push(event),
        )
        .expect("scan");

        let scanned = progress
            .iter()
            .rev()
            .find(|event| event.phase == ScanPhase::Scanning)
            .expect("scan progress");
        let aggregated = progress
            .iter()
            .rev()
            .find(|event| event.phase == ScanPhase::Aggregating)
            .expect("aggregation progress");

        assert_eq!(scanned.entries_total, 0);
        assert_eq!(scanned.entries_scanned, report.entries.len());
        assert_eq!(aggregated.entries_total, report.entries.len());
    }

    #[test]
    fn ignore_patterns_skip_matching_subtrees() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::create_dir(temp.path().join(".git")).expect("mkdir");
        fs::write(temp.path().join(".git").join("objects"), b"hidden").expect("write");
        fs::write(temp.path().join("visible"), b"visible").expect("write");

        let report = scan_paths(
            &[temp.path().to_path_buf()],
            &ScanOptions {
                ignore_patterns: vec!["**/.git".into(), "**/.git/**".into()],
                ..ScanOptions::default()
            },
        )
        .expect("scan");

        assert!(
            !report
                .entries
                .iter()
                .any(|entry| entry.path.ends_with(".git"))
        );
        assert!(
            report
                .entries
                .iter()
                .any(|entry| entry.path.ends_with("visible"))
        );
    }

    #[test]
    fn cancellable_scan_stops_before_completion() {
        use std::sync::atomic::AtomicBool;

        let temp = tempfile::tempdir().expect("tempdir");
        for index in 0..128 {
            fs::write(temp.path().join(format!("file-{index}")), b"x").expect("write");
        }
        let cancelled = AtomicBool::new(false);
        let result = scan_paths_with_progress_cancellable(
            &[temp.path().to_path_buf()],
            &ScanOptions::default(),
            &cancelled,
            |event| {
                if event.entries_scanned >= 4 {
                    cancelled.store(true, Ordering::Relaxed);
                }
            },
        );

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(SCAN_CANCELLED));
    }

    #[test]
    fn nested_and_duplicate_roots_are_scanned_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let nested = temp.path().join("nested");
        fs::create_dir(&nested).expect("nested dir");
        fs::write(nested.join("file"), b"123").expect("write");

        let report = scan_paths(
            &[
                nested.clone(),
                temp.path().to_path_buf(),
                temp.path().to_path_buf(),
            ],
            &ScanOptions::default(),
        )
        .expect("scan");

        assert_eq!(
            report.summary.roots,
            vec![temp.path().canonicalize().expect("root")]
        );
        assert_eq!(
            report
                .entries
                .iter()
                .filter(|entry| entry.path.ends_with("nested/file"))
                .count(),
            1
        );
    }

    #[test]
    fn explicit_ignore_directory_skips_the_entire_subtree() {
        let temp = tempfile::tempdir().expect("tempdir");
        let ignored = temp.path().join("ignored");
        fs::create_dir(&ignored).expect("ignored dir");
        fs::write(ignored.join("secret"), b"hidden").expect("write");
        fs::write(temp.path().join("visible"), b"visible").expect("write");

        let report = scan_paths(
            &[temp.path().to_path_buf()],
            &ScanOptions {
                ignore_dirs: vec![ignored],
                ..ScanOptions::default()
            },
        )
        .expect("scan");

        assert!(
            !report
                .entries
                .iter()
                .any(|entry| entry.path.ends_with("ignored"))
        );
        assert!(
            report
                .entries
                .iter()
                .any(|entry| entry.path.ends_with("visible"))
        );
    }

    #[test]
    fn invalid_ignore_glob_fails_before_scanning() {
        let error = scan_paths(
            &[PathBuf::from(".")],
            &ScanOptions {
                ignore_patterns: vec!["[".to_string()],
                ..ScanOptions::default()
            },
        )
        .expect_err("invalid glob");

        assert!(error.to_string().contains("invalid scan ignore pattern"));
    }

    #[cfg(unix)]
    #[test]
    fn hardlinked_files_are_counted_only_once() {
        let temp = tempfile::tempdir().expect("tempdir");
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::write(&first, b"123456").expect("write");
        fs::hard_link(&first, &second).expect("hard link");

        let report =
            scan_paths(&[temp.path().to_path_buf()], &ScanOptions::default()).expect("scan");
        let file_bytes = report
            .entries
            .iter()
            .filter(|entry| entry.kind == EntryKind::File)
            .map(|entry| entry.size_bytes)
            .sum::<u64>();

        assert_eq!(file_bytes, 6);
        assert_eq!(report.summary.total_size_bytes, 6);
    }
}
