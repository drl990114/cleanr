#![forbid(unsafe_code)]

use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{Context, Result, bail};
use cleanr_config::Config;
use cleanr_plugin_api::{
    PluginCapability, PluginDiagnostic, PluginDiscovery, discover_bundles, sorted_dir_entries,
};
use rust_i18n::t;
use serde::Deserialize;
use serde_yaml::Value;
use sha2::{Digest, Sha256};

rust_i18n::i18n!("locales", fallback = "en-US");

pub const FALLBACK_LOCALE: &str = "en-US";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LanguagePackInfo {
    pub id: String,
    pub locale: String,
    pub name: String,
    pub version: String,
    pub source: LanguagePackSource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LanguagePackSource {
    Builtin,
    UserFile(PathBuf),
    Plugin { id: String, path: PathBuf },
}

#[derive(Debug, Clone)]
pub struct I18n {
    locale: String,
    overlays: BTreeMap<String, BTreeMap<String, String>>,
    packs: Vec<LanguagePackInfo>,
    diagnostics: Vec<PluginDiagnostic>,
}

type ParsedLanguageFile = (BTreeMap<String, String>, Option<String>, Option<String>);

#[derive(Debug, Clone, Deserialize)]
struct Metadata {
    #[serde(rename = "_version")]
    schema_version: Option<u32>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    version: Option<String>,
}

impl I18n {
    pub fn load(config: &Config) -> Result<Self> {
        let discovery = discover_bundles(
            &config.plugins.dirs,
            &config.plugins.trusted,
            env!("CARGO_PKG_VERSION"),
        );
        Self::load_with_discovery(config, &discovery)
    }

    pub fn load_with_discovery(config: &Config, discovery: &PluginDiscovery) -> Result<Self> {
        let mut overlays = BTreeMap::new();
        let mut packs = builtin_language_packs();
        let mut diagnostics = discovery.diagnostics.clone();
        for bundle in &discovery.bundles {
            if !bundle
                .manifest
                .capabilities
                .contains(&PluginCapability::Translations)
            {
                continue;
            }
            let locales_dir = bundle.root.join("locales");
            let paths = match sorted_dir_entries(&locales_dir) {
                Ok(paths) => paths,
                Err(error) => {
                    diagnostics.push(PluginDiagnostic::warning(
                        "plugin-locales-directory-missing",
                        error.to_string(),
                        Some(locales_dir),
                    ));
                    continue;
                }
            };
            let paths = paths
                .into_iter()
                .filter(|path| is_language_file(path))
                .collect::<Vec<_>>();
            if paths.is_empty() {
                diagnostics.push(PluginDiagnostic::warning(
                    "plugin-locales-empty",
                    format!(
                        "plugin {} declares translations but contains no locale files",
                        bundle.manifest.id
                    ),
                    Some(locales_dir),
                ));
                continue;
            }
            for path in paths {
                if let Err(error) = load_language_file_if_absent(
                    &path,
                    &mut overlays,
                    &mut packs,
                    LanguagePackSource::Plugin {
                        id: bundle.manifest.id.clone(),
                        path: path.clone(),
                    },
                ) {
                    diagnostics.push(PluginDiagnostic::error(
                        "language-pack-invalid",
                        error.to_string(),
                        Some(path),
                    ));
                }
            }
        }

        for dir in &config.i18n.dirs {
            let paths = match sorted_dir_entries(dir) {
                Ok(paths) => paths,
                Err(_) => continue,
            };
            for path in paths.into_iter().filter(|path| is_language_file(path)) {
                if let Err(error) = load_language_file_if_absent(
                    &path,
                    &mut overlays,
                    &mut packs,
                    LanguagePackSource::UserFile(path.clone()),
                ) {
                    diagnostics.push(PluginDiagnostic::error(
                        "language-pack-invalid",
                        error.to_string(),
                        Some(path),
                    ));
                }
            }
        }

        let requested = config
            .i18n
            .locale
            .clone()
            .or_else(locale_from_env)
            .unwrap_or_else(|| FALLBACK_LOCALE.to_string());

        let mut i18n = Self::new(requested, overlays, packs);
        i18n.diagnostics = diagnostics;
        Ok(i18n)
    }

    #[must_use]
    pub fn new(
        requested_locale: impl Into<String>,
        overlays: BTreeMap<String, BTreeMap<String, String>>,
        packs: Vec<LanguagePackInfo>,
    ) -> Self {
        let locale = select_locale(&packs, &requested_locale.into())
            .unwrap_or_else(|| FALLBACK_LOCALE.to_string());
        rust_i18n::set_locale(&locale);
        Self {
            locale,
            overlays,
            packs,
            diagnostics: Vec::new(),
        }
    }

    #[must_use]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    pub fn set_locale(&mut self, locale: impl Into<String>) {
        let requested = locale.into();
        self.locale =
            select_locale(&self.packs, &requested).unwrap_or_else(|| FALLBACK_LOCALE.to_string());
        rust_i18n::set_locale(&self.locale);
    }

    #[must_use]
    pub fn packs(&self) -> &[LanguagePackInfo] {
        &self.packs
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[PluginDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn t(&self, key: &str) -> String {
        self.overlay_text(&self.locale, key)
            .or_else(|| self.overlay_text(&language_of(&self.locale), key))
            .or_else(|| self.overlay_text(FALLBACK_LOCALE, key))
            .or_else(|| self.overlay_text(&language_of(FALLBACK_LOCALE), key))
            .unwrap_or_else(|| builtin_t(&self.locale, key))
    }

    #[must_use]
    pub fn format(&self, key: &str, args: &[(&str, String)]) -> String {
        let mut text = self.t(key);
        for (name, value) in args {
            text = text.replace(&format!("{{{name}}}"), value);
        }
        text
    }

    fn overlay_text(&self, locale: &str, key: &str) -> Option<String> {
        self.overlays
            .get(&normalize_locale(locale))
            .and_then(|messages| messages.get(key))
            .cloned()
    }
}

pub fn load_language_dir(
    dir: impl AsRef<Path>,
    overlays: &mut BTreeMap<String, BTreeMap<String, String>>,
    packs: &mut Vec<LanguagePackInfo>,
) -> Result<()> {
    let dir = dir.as_ref();
    let Ok(entries) = sorted_dir_entries(dir) else {
        return Ok(());
    };

    for path in entries.into_iter().filter(|path| is_language_file(path)) {
        load_language_file(
            &path,
            overlays,
            packs,
            LanguagePackSource::UserFile(path.clone()),
        )?;
    }
    Ok(())
}

pub fn load_language_file(
    path: impl AsRef<Path>,
    overlays: &mut BTreeMap<String, BTreeMap<String, String>>,
    packs: &mut Vec<LanguagePackInfo>,
    source: LanguagePackSource,
) -> Result<()> {
    let path = path.as_ref();
    let locale = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(normalize_locale)
        .with_context(|| format!("language file {} has no locale file stem", path.display()))?;
    validate_locale(&locale)?;
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read language file {}", path.display()))?;
    let (messages, name, version) = parse_language_yaml(&raw)
        .with_context(|| format!("failed to parse language file {}", path.display()))?;
    if messages.is_empty() {
        bail!("language file {} contains no messages", path.display());
    }

    overlays.insert(locale.clone(), messages);
    upsert_pack(
        packs,
        LanguagePackInfo {
            id: format!("user-{locale}"),
            locale: locale.clone(),
            name: name.unwrap_or_else(|| locale.clone()),
            version: version.unwrap_or_else(|| "user".to_string()),
            source,
        },
    );
    Ok(())
}

pub fn install_github_language(
    locale: &str,
    repo: &str,
    reference: &str,
    output_dir: impl AsRef<Path>,
    expected_sha256: Option<&str>,
) -> Result<PathBuf> {
    let normalized = normalize_locale(locale);
    validate_locale(&normalized)?;
    if let Some(expected) = expected_sha256
        && (expected.len() != 64 || !expected.bytes().all(|byte| byte.is_ascii_hexdigit()))
    {
        bail!("expected language SHA-256 must contain exactly 64 hexadecimal characters");
    }
    let url = github_raw_language_url(repo, reference, &normalized)?;
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(Duration::from_secs(10))
        .timeout(Duration::from_secs(20))
        .build()
        .context("failed to create language download client")?;
    let response = client
        .get(&url)
        .send()
        .with_context(|| format!("failed to GET {url}"))?;
    if !response.status().is_success() {
        bail!("failed to download {url}: HTTP {}", response.status());
    }
    const MAX_LANGUAGE_BYTES: u64 = 1024 * 1024;
    if response
        .content_length()
        .is_some_and(|length| length > MAX_LANGUAGE_BYTES)
    {
        bail!("language file exceeds the 1 MiB size limit");
    }
    let mut body = Vec::new();
    response
        .take(MAX_LANGUAGE_BYTES + 1)
        .read_to_end(&mut body)
        .with_context(|| format!("failed to read response body from {url}"))?;
    if body.len() as u64 > MAX_LANGUAGE_BYTES {
        bail!("language file exceeds the 1 MiB size limit");
    }
    let body = String::from_utf8(body).context("language file is not valid UTF-8")?;
    parse_language_yaml(&body)
        .with_context(|| format!("downloaded language file {url} is invalid"))?;
    if let Some(expected) = expected_sha256 {
        let actual = format!("{:x}", Sha256::digest(body.as_bytes()));
        if !actual.eq_ignore_ascii_case(expected) {
            bail!("language file SHA-256 mismatch: expected {expected}, got {actual}");
        }
    }

    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let path = output_dir.join(format!("{normalized}.yml"));
    atomic_write(&path, body.as_bytes())?;
    Ok(path)
}

pub fn seed_builtin_language(locale: &str, output_dir: impl AsRef<Path>) -> Result<PathBuf> {
    let normalized = normalize_locale(locale);
    validate_locale(&normalized)?;
    let raw = builtin_locale_file(&normalized)
        .with_context(|| format!("no built-in language file for locale {normalized}"))?;
    let output_dir = output_dir.as_ref();
    fs::create_dir_all(output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let path = output_dir.join(format!("{normalized}.yml"));
    atomic_write(&path, raw.as_bytes())?;
    Ok(path)
}

fn atomic_write(path: &Path, contents: &[u8]) -> Result<()> {
    let directory = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(directory)
        .with_context(|| format!("failed to create {}", directory.display()))?;
    let mut temporary = tempfile::NamedTempFile::new_in(directory)
        .with_context(|| format!("failed to create temporary file in {}", directory.display()))?;
    temporary.write_all(contents)?;
    temporary.as_file().sync_all()?;
    temporary
        .persist(path)
        .map_err(|error| error.error)
        .with_context(|| format!("failed to replace {}", path.display()))?;
    Ok(())
}

pub fn github_raw_language_url(repo: &str, reference: &str, locale: &str) -> Result<String> {
    if repo.split('/').count() != 2
        || !repo
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'-' | b'_'))
    {
        bail!("GitHub repo must be in owner/name form");
    }
    if reference.is_empty()
        || !reference
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b'/'))
        || reference.contains("..")
    {
        bail!("GitHub reference contains unsupported characters");
    }
    let locale = normalize_locale(locale);
    validate_locale(&locale)?;
    Ok(format!(
        "https://raw.githubusercontent.com/{repo}/{reference}/crates/i18n/locales/{locale}.yml"
    ))
}

#[must_use]
pub fn builtin_language_packs() -> Vec<LanguagePackInfo> {
    vec![
        LanguagePackInfo {
            id: "builtin-en-us".to_string(),
            locale: "en-US".to_string(),
            name: "English (United States)".to_string(),
            version: "0.1.0".to_string(),
            source: LanguagePackSource::Builtin,
        },
        LanguagePackInfo {
            id: "builtin-zh-cn".to_string(),
            locale: "zh-CN".to_string(),
            name: "简体中文".to_string(),
            version: "0.1.0".to_string(),
            source: LanguagePackSource::Builtin,
        },
    ]
}

#[must_use]
pub fn builtin_locale_file(locale: &str) -> Option<&'static str> {
    match normalize_locale(locale).as_str() {
        "en-US" | "en" => Some(include_str!("../locales/en-US.yml")),
        "zh-CN" | "zh" => Some(include_str!("../locales/zh-CN.yml")),
        _ => None,
    }
}

#[must_use]
pub fn available_builtin_locales() -> Vec<&'static str> {
    vec!["en-US", "zh-CN"]
}

fn parse_language_yaml(raw: &str) -> Result<ParsedLanguageFile> {
    let value: Value = serde_yaml::from_str(raw)?;
    let Some(mapping) = value.as_mapping() else {
        bail!("language file root must be a YAML mapping");
    };

    let metadata = serde_yaml::from_value::<Metadata>(value.clone()).unwrap_or(Metadata {
        schema_version: None,
        name: None,
        version: None,
    });
    if metadata.schema_version != Some(1) {
        bail!("language file must declare _version: 1");
    }
    let mut messages = BTreeMap::new();
    flatten_mapping("", mapping, &mut messages)?;
    messages.retain(|key, _| !key.starts_with('_') && key != "name" && key != "version");
    Ok((messages, metadata.name, metadata.version))
}

pub fn validate_language_yaml(raw: &str) -> Result<()> {
    parse_language_yaml(raw).map(|_| ())
}

pub fn validate_language_file(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let locale = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(normalize_locale)
        .with_context(|| format!("language file {} has no locale file stem", path.display()))?;
    validate_locale(&locale)?;
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read language file {}", path.display()))?;
    validate_language_yaml(&raw)
        .with_context(|| format!("failed to validate language file {}", path.display()))
}

fn flatten_mapping(
    prefix: &str,
    mapping: &serde_yaml::Mapping,
    output: &mut BTreeMap<String, String>,
) -> Result<()> {
    for (key, value) in mapping {
        let Some(key) = key.as_str() else {
            continue;
        };
        let full_key = if prefix.is_empty() {
            key.to_string()
        } else {
            format!("{prefix}.{key}")
        };
        match value {
            Value::String(value) => {
                output.insert(full_key, value.clone());
            }
            Value::Mapping(nested) => flatten_mapping(&full_key, nested, output)?,
            Value::Null if full_key.starts_with('_') => {}
            _ if full_key.starts_with('_') || matches!(full_key.as_str(), "name" | "version") => {}
            _ => bail!("translation value {full_key} must be a string or mapping"),
        }
    }
    Ok(())
}

fn upsert_pack(packs: &mut Vec<LanguagePackInfo>, pack: LanguagePackInfo) {
    if let Some(existing) = packs
        .iter_mut()
        .find(|existing| existing.id == pack.id || same_locale(&existing.locale, &pack.locale))
    {
        *existing = pack;
    } else {
        packs.push(pack);
    }
}

fn load_language_file_if_absent(
    path: &Path,
    overlays: &mut BTreeMap<String, BTreeMap<String, String>>,
    packs: &mut Vec<LanguagePackInfo>,
    source: LanguagePackSource,
) -> Result<()> {
    let locale = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map(normalize_locale)
        .with_context(|| format!("language file {} has no locale file stem", path.display()))?;
    if overlays.contains_key(&locale) {
        bail!("duplicate language pack for locale {locale}; the first deterministic match wins");
    }
    load_language_file(path, overlays, packs, source)
}

fn is_language_file(path: &Path) -> bool {
    path.is_file()
        && matches!(
            path.extension().and_then(|extension| extension.to_str()),
            Some("yml" | "yaml")
        )
}

fn validate_locale(locale: &str) -> Result<()> {
    if locale.is_empty()
        || locale.len() > 35
        || locale.starts_with('-')
        || locale.ends_with('-')
        || !locale
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        bail!("invalid locale identifier: {locale}");
    }
    Ok(())
}

#[must_use]
pub fn language_pack_schema() -> serde_json::Value {
    serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "title": "Cleanr language pack",
        "type": "object",
        "required": ["_version"],
        "$defs": {
            "messageValue": {
                "oneOf": [
                    { "type": "string" },
                    {
                        "type": "object",
                        "additionalProperties": { "$ref": "#/$defs/messageValue" }
                    }
                ]
            }
        },
        "properties": {
            "_version": { "const": 1 },
            "name": { "type": "string" },
            "version": { "type": "string" }
        },
        "additionalProperties": { "$ref": "#/$defs/messageValue" }
    })
}

fn select_locale(packs: &[LanguagePackInfo], requested_locale: &str) -> Option<String> {
    let requested = normalize_locale(requested_locale);
    packs
        .iter()
        .find(|pack| same_locale(&pack.locale, &requested))
        .or_else(|| {
            let requested_language = language_of(&requested);
            packs
                .iter()
                .find(|pack| language_of(&pack.locale) == requested_language)
        })
        .map(|pack| pack.locale.clone())
}

fn same_locale(a: &str, b: &str) -> bool {
    normalize_locale(a) == normalize_locale(b)
}

fn language_of(locale: &str) -> String {
    normalize_locale(locale).split_once('-').map_or_else(
        || normalize_locale(locale),
        |(language, _)| language.to_string(),
    )
}

fn normalize_locale(locale: &str) -> String {
    let base = locale
        .split('.')
        .next()
        .unwrap_or(locale)
        .split('@')
        .next()
        .unwrap_or(locale)
        .replace('_', "-");
    let mut parts = base.split('-');
    let language = parts.next().unwrap_or_default().to_ascii_lowercase();
    let region = parts.next().map(str::to_ascii_uppercase);
    match region {
        Some(region) if !region.is_empty() => format!("{language}-{region}"),
        _ => language,
    }
}

fn locale_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .into_iter()
        .find_map(|name| {
            let value = std::env::var(name).ok()?;
            (!value.is_empty() && value != "C" && value != "POSIX").then_some(value)
        })
}

fn builtin_t(locale: &str, key: &str) -> String {
    macro_rules! tr {
        ($literal:literal) => {
            t!($literal, locale = locale).to_string()
        };
    }

    match key {
        "status_ready" => tr!("status_ready"),
        "status_update_available" => tr!("status_update_available"),
        "status_home" => tr!("status_home"),
        "status_queued" => tr!("status_queued"),
        "status_plain_language_scan_review" => tr!("status_plain_language_scan_review"),
        "status_plain_language_help" => tr!("status_plain_language_help"),
        "status_no_selected_items" => tr!("status_no_selected_items"),
        "status_clean_confirm" => tr!("status_clean_confirm"),
        "status_clean_cancelled" => tr!("status_clean_cancelled"),
        "status_cleaned" => tr!("status_cleaned"),
        "status_restore_confirm" => tr!("status_restore_confirm"),
        "status_restore_cancelled" => tr!("status_restore_cancelled"),
        "status_restore_already_done" => tr!("status_restore_already_done"),
        "status_restored" => tr!("status_restored"),
        "status_scan_finished" => tr!("status_scan_finished"),
        "status_scan_disconnected" => tr!("status_scan_disconnected"),
        "status_scan_already_running" => tr!("status_scan_already_running"),
        "status_scan_cancelling" => tr!("status_scan_cancelling"),
        "status_scan_cancelled" => tr!("status_scan_cancelled"),
        "status_no_global_caches" => tr!("status_no_global_caches"),
        "status_scanning" => tr!("status_scanning"),
        "status_scan_progress" => tr!("status_scan_progress"),
        "status_scan_progress_unbounded" => tr!("status_scan_progress_unbounded"),
        "status_scan_started" => tr!("status_scan_started"),
        "status_scan_log" => tr!("status_scan_log"),
        "status_review_after_scan" => tr!("status_review_after_scan"),
        "status_no_scan_results" => tr!("status_no_scan_results"),
        "status_plan_ready" => tr!("status_plan_ready"),
        "status_exported_plan" => tr!("status_exported_plan"),
        "status_no_manifests" => tr!("status_no_manifests"),
        "status_latest_run" => tr!("status_latest_run"),
        "status_rules" => tr!("status_rules"),
        "status_plugins" => tr!("status_plugins"),
        "status_languages" => tr!("status_languages"),
        "status_language_switched" => tr!("status_language_switched"),
        "status_no_tasks" => tr!("status_no_tasks"),
        "status_usage" => tr!("status_usage"),
        "status_select_scan_only" => tr!("status_select_scan_only"),
        "status_help" => tr!("status_help"),
        "status_all_toggled_selected" => tr!("status_all_toggled_selected"),
        "status_all_toggled_deselected" => tr!("status_all_toggled_deselected"),
        "label_status" => tr!("label_status"),
        "label_roots" => tr!("label_roots"),
        "label_candidates" => tr!("label_candidates"),
        "label_scan_tree" => tr!("label_scan_tree"),
        "label_cleanup_plan" => tr!("label_cleanup_plan"),
        "label_command" => tr!("label_command"),
        "label_slash_commands" => tr!("label_slash_commands"),
        "label_safety" => tr!("label_safety"),
        "label_preview" => tr!("label_preview"),
        "label_home" => tr!("label_home"),
        "label_dashboard" => tr!("label_dashboard"),
        "label_languages" => tr!("label_languages"),
        "home_overview" => tr!("home_overview"),
        "label_rules" => tr!("label_rules"),
        "label_plugins" => tr!("label_plugins"),
        "label_tasks" => tr!("label_tasks"),
        "label_usage" => tr!("label_usage"),
        "label_restore" => tr!("label_restore"),
        "label_details" => tr!("label_details"),
        "label_mode_normal" => tr!("label_mode_normal"),
        "label_mode_command" => tr!("label_mode_command"),
        "label_help" => tr!("label_help"),
        "home_welcome" => tr!("home_welcome"),
        "home_subtitle" => tr!("home_subtitle"),
        "home_roots" => tr!("home_roots"),
        "home_hint" => tr!("home_hint"),
        "home_no_scan" => tr!("home_no_scan"),
        "home_last_scan" => tr!("home_last_scan"),
        "home_plan_ready" => tr!("home_plan_ready"),
        "home_plan_waiting" => tr!("home_plan_waiting"),
        "home_quick_commands" => tr!("home_quick_commands"),
        "home_action_scan" => tr!("home_action_scan"),
        "home_action_usage" => tr!("home_action_usage"),
        "home_action_more" => tr!("home_action_more"),
        "home_action_review" => tr!("home_action_review"),
        "home_action_rescan" => tr!("home_action_rescan"),
        "home_safety_note" => tr!("home_safety_note"),
        "home_result_title" => tr!("home_result_title"),
        "home_result_empty" => tr!("home_result_empty"),
        "home_result_scanned" => tr!("home_result_scanned"),
        "home_result_reclaimable" => tr!("home_result_reclaimable"),
        "home_result_candidates" => tr!("home_result_candidates"),
        "home_result_selected" => tr!("home_result_selected"),
        "home_session" => tr!("home_session"),
        "home_recent_activity" => tr!("home_recent_activity"),
        "home_recent_empty" => tr!("home_recent_empty"),
        "language_home_hint" => tr!("language_home_hint"),
        "plugins_context_hint" => tr!("plugins_context_hint"),
        "usage_context_hint" => tr!("usage_context_hint"),
        "usage_overview" => tr!("usage_overview"),
        "usage_metric_total" => tr!("usage_metric_total"),
        "usage_metric_entries" => tr!("usage_metric_entries"),
        "usage_metric_candidates" => tr!("usage_metric_candidates"),
        "usage_metric_selected" => tr!("usage_metric_selected"),
        "scan_phase_discovering" => tr!("scan_phase_discovering"),
        "scan_phase_scanning" => tr!("scan_phase_scanning"),
        "scan_phase_aggregating" => tr!("scan_phase_aggregating"),
        "scan_progress_discovered" => tr!("scan_progress_discovered"),
        "scan_progress_count" => tr!("scan_progress_count"),
        "scan_progress_unbounded" => tr!("scan_progress_unbounded"),
        "scan_progress_aggregating" => tr!("scan_progress_aggregating"),
        "scan_progress_stats" => tr!("scan_progress_stats"),
        "scan_preparing" => tr!("scan_preparing"),
        "scan_current_path" => tr!("scan_current_path"),
        "scan_cancel_hint" => tr!("scan_cancel_hint"),
        "help_title" => tr!("help_title"),
        "help_move" => tr!("help_move"),
        "help_select_all" => tr!("help_select_all"),
        "help_toggle" => tr!("help_toggle"),
        "help_actions" => tr!("help_actions"),
        "help_command" => tr!("help_command"),
        "help_palette" => tr!("help_palette"),
        "help_page" => tr!("help_page"),
        "help_home" => tr!("help_home"),
        "help_confirm_yes" => tr!("help_confirm_yes"),
        "help_confirm_no" => tr!("help_confirm_no"),
        "help_quit" => tr!("help_quit"),
        "status_item_toggled" => tr!("status_item_toggled"),
        "state_selected" => tr!("state_selected"),
        "state_deselected" => tr!("state_deselected"),
        "command_placeholder" => tr!("command_placeholder"),
        "hint_scan" => tr!("hint_scan"),
        "hint_usage" => tr!("hint_usage"),
        "hint_move" => tr!("hint_move"),
        "hint_select" => tr!("hint_select"),
        "hint_clean" => tr!("hint_clean"),
        "hint_commands" => tr!("hint_commands"),
        "hint_help" => tr!("hint_help"),
        "hint_quit" => tr!("hint_quit"),
        "hint_choose" => tr!("hint_choose"),
        "hint_run" => tr!("hint_run"),
        "hint_close" => tr!("hint_close"),
        "confirm_title" => tr!("confirm_title"),
        "confirm_body" => tr!("confirm_body"),
        "confirm_restore_title" => tr!("confirm_restore_title"),
        "confirm_restore_body" => tr!("confirm_restore_body"),
        "confirm_yes" => tr!("confirm_yes"),
        "confirm_no" => tr!("confirm_no"),
        "confirm_hint" => tr!("confirm_hint"),
        "plan_schema" => tr!("plan_schema"),
        "plan_candidates" => tr!("plan_candidates"),
        "plan_selected" => tr!("plan_selected"),
        "plan_selected_size" => tr!("plan_selected_size"),
        "plan_default_action" => tr!("plan_default_action"),
        "plan_requires_confirmation" => tr!("plan_requires_confirmation"),
        "plan_agent_can_execute" => tr!("plan_agent_can_execute"),
        "plan_rollback" => tr!("plan_rollback"),
        "plan_export_hint" => tr!("plan_export_hint"),
        "plan_clean_hint" => tr!("plan_clean_hint"),
        "plan_scanning" => tr!("plan_scanning"),
        "plan_keep_typing" => tr!("plan_keep_typing"),
        "plan_empty" => tr!("plan_empty"),
        "plan_empty_hint" => tr!("plan_empty_hint"),
        "command_scan" => tr!("command_scan"),
        "command_review" => tr!("command_review"),
        "command_plan" => tr!("command_plan"),
        "command_clean" => tr!("command_clean"),
        "command_clean_confirm" => tr!("command_clean_confirm"),
        "command_restore" => tr!("command_restore"),
        "command_rules" => tr!("command_rules"),
        "command_plugins" => tr!("command_plugins"),
        "command_languages" => tr!("command_languages"),
        "command_tasks" => tr!("command_tasks"),
        "command_usage" => tr!("command_usage"),
        "command_export_plan" => tr!("command_export_plan"),
        "command_help" => tr!("command_help"),
        "command_quit" => tr!("command_quit"),
        "restore_select_hint" => tr!("restore_select_hint"),
        "restore_state_available" => tr!("restore_state_available"),
        "restore_state_restored" => tr!("restore_state_restored"),
        _ => key.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selects_builtin_chinese_locale() {
        let i18n = I18n::new("zh_CN.UTF-8", BTreeMap::new(), builtin_language_packs());
        assert_eq!(i18n.locale(), "zh-CN");
        assert_eq!(i18n.t("label_status"), "状态");
    }

    #[test]
    fn loads_language_pack_plugins_from_directory() {
        let temp = tempfile::tempdir().expect("tempdir");
        let pack_path = temp.path().join("en-XA.yml");
        fs::write(
            &pack_path,
            r#"
_version: 1
name: Pirate
version: 0.1.0
label_status: Ship
"#,
        )
        .expect("write pack");

        let mut overlays = BTreeMap::new();
        let mut packs = builtin_language_packs();
        load_language_dir(temp.path(), &mut overlays, &mut packs).expect("load dir");
        let i18n = I18n::new("en-XA", overlays, packs);
        assert_eq!(i18n.t("label_status"), "Ship");
        assert_eq!(i18n.t("label_command"), "Command");
    }

    #[test]
    fn builds_github_raw_language_url() {
        let url = github_raw_language_url("owner/repo", "main", "zh_CN").expect("url");
        assert_eq!(
            url,
            "https://raw.githubusercontent.com/owner/repo/main/crates/i18n/locales/zh-CN.yml"
        );
    }

    #[test]
    fn duplicate_locales_keep_the_first_configured_directory() {
        let first = tempfile::tempdir().expect("first tempdir");
        let second = tempfile::tempdir().expect("second tempdir");
        for (directory, label) in [(&first, "First"), (&second, "Second")] {
            fs::write(
                directory.path().join("en-XA.yml"),
                format!("_version: 1\nname: Test\nversion: 1.0.0\nlabel_status: {label}\n"),
            )
            .expect("write language pack");
        }
        let mut config = Config::default();
        config.plugins.dirs.clear();
        config.i18n.dirs = vec![first.path().to_path_buf(), second.path().to_path_buf()];
        config.i18n.locale = Some("en-XA".to_string());

        let i18n = I18n::load(&config).expect("load i18n");

        assert_eq!(i18n.t("label_status"), "First");
        assert!(
            i18n.diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "language-pack-invalid")
        );
    }

    #[test]
    fn invalid_language_pack_is_diagnostic_not_startup_failure() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("en-XA.yml"),
            "_version: 2\nlabel_status: Bad\n",
        )
        .expect("write invalid language pack");
        let mut config = Config::default();
        config.plugins.dirs.clear();
        config.i18n.dirs = vec![temp.path().to_path_buf()];

        let i18n = I18n::load(&config).expect("load i18n");

        assert_eq!(i18n.diagnostics().len(), 1);
        assert_eq!(i18n.diagnostics()[0].code, "language-pack-invalid");
    }

    #[test]
    fn rejects_malformed_sha256_before_downloading() {
        let error =
            install_github_language("en-US", "owner/repo", "main", ".", Some("not-a-sha256"))
                .expect_err("invalid hash must fail");
        assert!(error.to_string().contains("64 hexadecimal"));
    }
}
