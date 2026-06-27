#![forbid(unsafe_code)]

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use anyhow::{Context, Result, bail};
use chrono::{Duration, Utc};
use cleanr_config::Config;
use cleanr_core::{Confidence, EntryKind, RuleHit, RuleTrust, RulesetVersion, ScanEntry};
use cleanr_plugin_api::{
    PluginCapability, PluginDiagnostic, PluginDiscovery, PluginManifest, PluginSource, TrustLevel,
    discover_bundles, sorted_dir_entries,
};
use globset::{Glob, GlobMatcher};
use schemars::{JsonSchema, schema_for};
use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RulePack {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub categories: Vec<String>,
    pub rules: Vec<RuleDefinition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleDefinition {
    pub id: String,
    pub label: String,
    pub category: String,
    #[serde(rename = "match")]
    pub matcher: RuleMatcher,
    pub confidence: Confidence,
    pub default_selected: bool,
    pub action: RuleAction,
    pub reason: String,
    pub risk_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RuleMatcher {
    pub dir_name: Option<String>,
    pub path_glob: Option<String>,
    pub file_name: Option<String>,
    pub extension: Option<String>,
    pub max_age_days: Option<i64>,
    pub min_size: Option<u64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum RuleAction {
    Trash,
}

#[derive(Debug, Clone)]
pub struct LoadedRulePack {
    pub definition: RulePack,
    pub source: PluginSource,
    pub trust: TrustLevel,
    pub plugin_id: Option<String>,
    compiled_rules: Vec<CompiledRule>,
}

#[derive(Debug, Clone)]
struct CompiledRule {
    path_glob: Option<GlobMatcher>,
}

#[derive(Debug, Clone)]
pub struct RuleRegistry {
    packs: Vec<LoadedRulePack>,
    diagnostics: Vec<PluginDiagnostic>,
    dir_name_index: BTreeMap<String, Vec<(usize, usize)>>,
    file_name_index: BTreeMap<String, Vec<(usize, usize)>>,
    extension_index: BTreeMap<String, Vec<(usize, usize)>>,
    generic_rules: Vec<(usize, usize)>,
}

impl RulePack {
    pub fn from_toml(raw: &str) -> Result<Self> {
        let pack: Self = toml::from_str(raw).context("failed to parse rule pack TOML")?;
        pack.validate()?;
        Ok(pack)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            bail!("rule pack id cannot be empty");
        }
        if self.version.trim().is_empty() {
            bail!("rule pack {} has an empty version", self.id);
        }
        Version::parse(&self.version)
            .with_context(|| format!("rule pack {} has an invalid semantic version", self.id))?;
        if self.rules.is_empty() {
            bail!("rule pack {} contains no rules", self.id);
        }
        let mut rule_ids = BTreeSet::new();
        for rule in &self.rules {
            if rule.id.trim().is_empty() {
                bail!("rule pack {} contains a rule with an empty id", self.id);
            }
            if rule.default_selected && rule.confidence != Confidence::High {
                bail!(
                    "rule {}:{} cannot be default_selected unless confidence is high",
                    self.id,
                    rule.id
                );
            }
            if !rule_ids.insert(rule.id.as_str()) {
                bail!(
                    "rule pack {} contains duplicate rule id {}",
                    self.id,
                    rule.id
                );
            }
            if !self
                .categories
                .iter()
                .any(|category| category == &rule.category)
            {
                bail!(
                    "rule {}:{} uses undeclared category {}",
                    self.id,
                    rule.id,
                    rule.category
                );
            }
            if rule.matcher.max_age_days.is_some_and(|days| days < 0) {
                bail!("rule {}:{} has a negative max_age_days", self.id, rule.id);
            }
            let has_matcher = rule.matcher.dir_name.is_some()
                || rule.matcher.path_glob.is_some()
                || rule.matcher.file_name.is_some()
                || rule.matcher.extension.is_some()
                || rule.matcher.max_age_days.is_some()
                || rule.matcher.min_size.is_some();
            if !has_matcher {
                bail!("rule {}:{} has no matcher", self.id, rule.id);
            }
            if let Some(path_glob) = &rule.matcher.path_glob {
                Glob::new(path_glob).with_context(|| {
                    format!("rule {}:{} has an invalid path_glob", self.id, rule.id)
                })?;
            }
        }
        Ok(())
    }
}

impl RuleRegistry {
    pub fn builtin() -> Result<Self> {
        let mut registry = Self::empty();
        registry.add_builtin_plugin(BUILTIN_DEV_MANIFEST, &[BUILTIN_DEV_RULES])?;
        registry.add_builtin_plugin(BUILTIN_GENERAL_MANIFEST, &[BUILTIN_GENERAL_RULES])?;
        Ok(registry)
    }

    pub fn load(config: &Config) -> Result<Self> {
        let discovery = discover_bundles(
            &config.plugins.dirs,
            &config.plugins.trusted,
            env!("CARGO_PKG_VERSION"),
        );
        Self::load_with_discovery(config, &discovery)
    }

    pub fn load_with_discovery(config: &Config, discovery: &PluginDiscovery) -> Result<Self> {
        let mut registry = Self::builtin()?;
        registry
            .diagnostics
            .extend(discovery.diagnostics.iter().cloned());

        for bundle in &discovery.bundles {
            if bundle
                .manifest
                .capabilities
                .contains(&PluginCapability::DynamicCandidates)
            {
                registry.diagnostics.push(PluginDiagnostic::warning(
                    "dynamic-candidates-runtime-disabled",
                    format!(
                        "plugin {} declares dynamic-candidates, but hook execution is not enabled in this release",
                        bundle.manifest.id
                    ),
                    Some(bundle.root.clone()),
                ));
            }
            if !bundle
                .manifest
                .capabilities
                .contains(&PluginCapability::Rules)
            {
                continue;
            }
            let rules_dir = bundle.root.join("rules");
            let paths = match sorted_dir_entries(&rules_dir) {
                Ok(paths) => paths,
                Err(error) => {
                    registry.diagnostics.push(PluginDiagnostic::warning(
                        "plugin-rules-directory-missing",
                        error.to_string(),
                        Some(rules_dir),
                    ));
                    continue;
                }
            };
            let paths = paths
                .into_iter()
                .filter(|path| is_toml_file(path))
                .collect::<Vec<_>>();
            if paths.is_empty() {
                registry.diagnostics.push(PluginDiagnostic::warning(
                    "plugin-rules-empty",
                    format!(
                        "plugin {} declares rules but contains no rule packs",
                        bundle.manifest.id
                    ),
                    Some(rules_dir),
                ));
                continue;
            }
            for path in paths {
                registry.load_user_pack(
                    &path,
                    bundle.trust,
                    Some(bundle.manifest.id.clone()),
                    PluginSource::Bundle(bundle.root.clone()),
                );
            }
        }

        for dir in &config.plugins.dirs {
            registry.load_legacy_dir(dir, &config.plugins.trusted);
        }

        let loaded_ids = registry
            .packs
            .iter()
            .map(|pack| pack.definition.id.clone())
            .collect::<BTreeSet<_>>();
        for enabled in &config.cleanup.enabled_rule_packs {
            if !loaded_ids.contains(enabled) {
                registry.diagnostics.push(PluginDiagnostic::warning(
                    "rule-pack-not-found",
                    format!("enabled rule pack {enabled} was not found"),
                    None,
                ));
            }
        }
        registry.packs.retain(|pack| {
            config
                .cleanup
                .enabled_rule_packs
                .iter()
                .any(|enabled| enabled == &pack.definition.id)
        });
        registry.rebuild_indexes();
        Ok(registry)
    }

    pub fn load_dir(&mut self, dir: impl AsRef<Path>) -> Result<()> {
        self.load_legacy_dir(dir.as_ref(), &[]);
        Ok(())
    }

    #[must_use]
    pub fn packs(&self) -> &[LoadedRulePack] {
        &self.packs
    }

    #[must_use]
    pub fn diagnostics(&self) -> &[PluginDiagnostic] {
        &self.diagnostics
    }

    #[must_use]
    pub fn versions(&self) -> Vec<RulesetVersion> {
        self.packs
            .iter()
            .map(|pack| RulesetVersion {
                id: pack.definition.id.clone(),
                version: pack.definition.version.clone(),
            })
            .collect()
    }

    pub fn annotate_entries(&self, entries: &mut [ScanEntry]) {
        for entry in entries {
            entry.rule_hits = self.hits_for(entry);
        }
    }

    #[must_use]
    pub fn hits_for(&self, entry: &ScanEntry) -> Vec<RuleHit> {
        let mut candidates = self.generic_rules.iter().copied().collect::<BTreeSet<_>>();
        if let Some(name) = entry.file_name() {
            if entry.kind == EntryKind::Directory {
                candidates.extend(
                    self.dir_name_index
                        .get(&name)
                        .into_iter()
                        .flatten()
                        .copied(),
                );
            } else {
                candidates.extend(
                    self.file_name_index
                        .get(&name)
                        .into_iter()
                        .flatten()
                        .copied(),
                );
            }
        }
        if let Some(extension) = entry.path.extension().and_then(|value| value.to_str()) {
            candidates.extend(
                self.extension_index
                    .get(&extension.to_ascii_lowercase())
                    .into_iter()
                    .flatten()
                    .copied(),
            );
        }

        candidates
            .into_iter()
            .filter_map(|(pack_index, rule_index)| {
                let pack = self.packs.get(pack_index)?;
                let rule = pack.definition.rules.get(rule_index)?;
                let compiled = pack.compiled_rules.get(rule_index)?;
                matches_rule(entry, rule, compiled).then(|| RuleHit {
                    rule_pack_id: pack.definition.id.clone(),
                    rule_id: rule.id.clone(),
                    label: rule.label.clone(),
                    category: rule.category.clone(),
                    confidence: rule.confidence,
                    reason: rule.reason.clone(),
                    risk_note: rule.risk_note.clone(),
                    default_selected: rule.default_selected,
                    trust: match pack.trust {
                        TrustLevel::Builtin => RuleTrust::Builtin,
                        TrustLevel::Trusted => RuleTrust::Trusted,
                        TrustLevel::Untrusted => RuleTrust::Untrusted,
                    },
                })
            })
            .collect()
    }

    fn empty() -> Self {
        Self {
            packs: Vec::new(),
            diagnostics: Vec::new(),
            dir_name_index: BTreeMap::new(),
            file_name_index: BTreeMap::new(),
            extension_index: BTreeMap::new(),
            generic_rules: Vec::new(),
        }
    }

    fn add_pack(
        &mut self,
        pack: RulePack,
        source: PluginSource,
        trust: TrustLevel,
        plugin_id: Option<String>,
    ) -> Result<()> {
        if self
            .packs
            .iter()
            .any(|loaded| loaded.definition.id == pack.id)
        {
            bail!("duplicate rule pack id {}", pack.id);
        }
        let compiled_rules = pack
            .rules
            .iter()
            .map(|rule| {
                rule.matcher
                    .path_glob
                    .as_deref()
                    .map(Glob::new)
                    .transpose()
                    .map(|glob| CompiledRule {
                        path_glob: glob.map(|glob| glob.compile_matcher()),
                    })
            })
            .collect::<std::result::Result<Vec<_>, _>>()?;
        if trust == TrustLevel::Untrusted && pack.rules.iter().any(|rule| rule.default_selected) {
            self.diagnostics.push(PluginDiagnostic::warning(
                "untrusted-default-selection-disabled",
                format!(
                    "rule pack {} requested default selection, but it is not trusted",
                    pack.id
                ),
                source.path().map(Path::to_path_buf),
            ));
        }
        self.packs.push(LoadedRulePack {
            definition: pack,
            source,
            trust,
            plugin_id,
            compiled_rules,
        });
        self.rebuild_indexes();
        Ok(())
    }

    fn add_builtin_plugin(&mut self, manifest_raw: &str, rules: &[&str]) -> Result<()> {
        let manifest = PluginManifest::from_toml(manifest_raw, env!("CARGO_PKG_VERSION"))?;
        if !manifest.capabilities.contains(&PluginCapability::Rules) {
            bail!("built-in plugin {} does not provide rules", manifest.id);
        }
        for raw in rules {
            self.add_pack(
                RulePack::from_toml(raw)?,
                PluginSource::Builtin,
                TrustLevel::Builtin,
                Some(manifest.id.clone()),
            )?;
        }
        Ok(())
    }

    fn load_user_pack(
        &mut self,
        path: &Path,
        trust: TrustLevel,
        plugin_id: Option<String>,
        source: PluginSource,
    ) {
        let result = fs::read_to_string(path)
            .with_context(|| format!("failed to read rule plugin {}", path.display()))
            .and_then(|raw| RulePack::from_toml(&raw))
            .and_then(|pack| self.add_pack(pack, source, trust, plugin_id));
        if let Err(error) = result {
            self.diagnostics.push(PluginDiagnostic::error(
                "rule-pack-invalid",
                error.to_string(),
                Some(path.to_path_buf()),
            ));
        }
    }

    fn load_legacy_dir(&mut self, dir: &Path, trusted_ids: &[String]) {
        let paths = match sorted_dir_entries(dir) {
            Ok(paths) => paths,
            Err(_) => return,
        };
        for path in paths.into_iter().filter(|path| is_toml_file(path)) {
            if path.file_name().and_then(|name| name.to_str()) == Some("plugin.toml") {
                continue;
            }
            let raw = match fs::read_to_string(&path) {
                Ok(raw) => raw,
                Err(error) => {
                    self.diagnostics.push(PluginDiagnostic::error(
                        "rule-pack-read-failed",
                        error.to_string(),
                        Some(path),
                    ));
                    continue;
                }
            };
            let pack = match RulePack::from_toml(&raw) {
                Ok(pack) => pack,
                Err(error) => {
                    self.diagnostics.push(PluginDiagnostic::error(
                        "rule-pack-invalid",
                        error.to_string(),
                        Some(path),
                    ));
                    continue;
                }
            };
            let trust = if trusted_ids.iter().any(|trusted| trusted == &pack.id) {
                TrustLevel::Trusted
            } else {
                TrustLevel::Untrusted
            };
            if let Err(error) =
                self.add_pack(pack, PluginSource::LegacyFile(path.clone()), trust, None)
            {
                self.diagnostics.push(PluginDiagnostic::error(
                    "rule-pack-invalid",
                    error.to_string(),
                    Some(path),
                ));
            }
        }
    }

    fn rebuild_indexes(&mut self) {
        self.dir_name_index.clear();
        self.file_name_index.clear();
        self.extension_index.clear();
        self.generic_rules.clear();
        for (pack_index, pack) in self.packs.iter().enumerate() {
            for (rule_index, rule) in pack.definition.rules.iter().enumerate() {
                let key = (pack_index, rule_index);
                if let Some(name) = &rule.matcher.dir_name {
                    self.dir_name_index
                        .entry(name.clone())
                        .or_default()
                        .push(key);
                } else if let Some(name) = &rule.matcher.file_name {
                    self.file_name_index
                        .entry(name.clone())
                        .or_default()
                        .push(key);
                } else if let Some(extension) = &rule.matcher.extension {
                    self.extension_index
                        .entry(extension.to_ascii_lowercase())
                        .or_default()
                        .push(key);
                } else {
                    self.generic_rules.push(key);
                }
            }
        }
    }
}

fn matches_rule(entry: &ScanEntry, rule: &RuleDefinition, compiled: &CompiledRule) -> bool {
    let matcher = &rule.matcher;
    if let Some(dir_name) = &matcher.dir_name
        && (entry.kind != EntryKind::Directory || entry.file_name().as_deref() != Some(dir_name))
    {
        return false;
    }
    if let Some(file_name) = &matcher.file_name
        && (entry.kind == EntryKind::Directory || entry.file_name().as_deref() != Some(file_name))
    {
        return false;
    }
    if let Some(extension) = &matcher.extension
        && entry
            .path
            .extension()
            .map(|ext| ext.to_string_lossy().eq_ignore_ascii_case(extension))
            != Some(true)
    {
        return false;
    }
    if let Some(min_size) = matcher.min_size
        && entry.size_bytes < min_size
    {
        return false;
    }
    if let Some(max_age_days) = matcher.max_age_days {
        let Some(modified_at) = entry.modified_at else {
            return false;
        };
        if modified_at > Utc::now() - Duration::days(max_age_days) {
            return false;
        }
    }
    if let Some(matcher) = &compiled.path_glob {
        let path = normalized_path(&entry.path);
        if !matcher.is_match(path) {
            return false;
        }
    }
    true
}

fn is_toml_file(path: &Path) -> bool {
    path.is_file() && path.extension().and_then(|extension| extension.to_str()) == Some("toml")
}

#[must_use]
pub fn rule_pack_schema() -> serde_json::Value {
    serde_json::to_value(schema_for!(RulePack)).expect("rule pack schema serializes")
}

fn normalized_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

const BUILTIN_DEV_MANIFEST: &str = include_str!("../builtin-plugins/builtin-dev/plugin.toml");
const BUILTIN_DEV_RULES: &str = include_str!("../builtin-plugins/builtin-dev/rules/dev.toml");
const BUILTIN_GENERAL_MANIFEST: &str =
    include_str!("../builtin-plugins/builtin-general/plugin.toml");
const BUILTIN_GENERAL_RULES: &str =
    include_str!("../builtin-plugins/builtin-general/rules/general.toml");

#[cfg(test)]
mod tests {
    use super::*;
    use cleanr_core::{RuleTrust, ScanEntry, build_cleanup_plan};
    use std::path::PathBuf;

    #[test]
    fn builtin_rules_match_developer_caches() {
        let registry = RuleRegistry::builtin().expect("builtin rules load");
        let entry = ScanEntry {
            path: PathBuf::from("/repo/node_modules"),
            kind: EntryKind::Directory,
            size_bytes: 2 * 1024 * 1024,
            modified_at: None,
            rule_hits: vec![],
        };

        let hits = registry.hits_for(&entry);
        assert_eq!(hits[0].rule_id, "node-modules");
        assert!(hits[0].default_selected);
        assert_eq!(hits[0].confidence, Confidence::High);
    }

    #[test]
    fn plugin_rejects_default_selected_non_high_confidence() {
        let raw = r#"
        id = "bad"
        name = "Bad"
        version = "0.1.0"
        description = "Bad"
        categories = ["x"]

        [[rules]]
        id = "bad-rule"
        label = "Bad"
        category = "x"
        match = { dir_name = "x" }
        confidence = "low"
        default_selected = true
        action = "trash"
        reason = "x"
        risk_note = "x"
        "#;

        assert!(RulePack::from_toml(raw).is_err());
    }

    #[test]
    fn duplicate_rule_packs_keep_the_first_sorted_file() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(temp.path().join("b.toml"), test_rule_pack("duplicate", "B")).expect("write b");
        fs::write(temp.path().join("a.toml"), test_rule_pack("duplicate", "A")).expect("write a");
        let mut config = Config::default();
        config.plugins.dirs = vec![temp.path().to_path_buf()];
        config.cleanup.enabled_rule_packs = vec!["duplicate".to_string()];

        let registry = RuleRegistry::load(&config).expect("load registry");

        assert_eq!(registry.packs().len(), 1);
        assert_eq!(registry.packs()[0].definition.name, "A");
        assert!(
            registry
                .diagnostics()
                .iter()
                .any(|diagnostic| diagnostic.code == "rule-pack-invalid")
        );
    }

    #[test]
    fn untrusted_rules_never_preselect_cleanup_items() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("custom.toml"),
            test_rule_pack("custom", "Custom"),
        )
        .expect("write custom rule");
        let mut config = Config::default();
        config.plugins.dirs = vec![temp.path().to_path_buf()];
        config.cleanup.enabled_rule_packs = vec!["custom".to_string()];
        let registry = RuleRegistry::load(&config).expect("load registry");
        let mut entry = ScanEntry {
            path: PathBuf::from("/repo/target"),
            kind: EntryKind::Directory,
            size_bytes: 1,
            modified_at: None,
            rule_hits: vec![],
        };
        entry.rule_hits = registry.hits_for(&entry);

        let plan = build_cleanup_plan(vec![PathBuf::from("/repo")], registry.versions(), &[entry]);

        assert_eq!(plan.summary.candidate_count, 1);
        assert_eq!(plan.summary.selected_count, 0);
        assert_eq!(
            registry.hits_for(&ScanEntry {
                path: PathBuf::from("/repo/target"),
                kind: EntryKind::Directory,
                size_bytes: 1,
                modified_at: None,
                rule_hits: vec![],
            })[0]
                .trust,
            RuleTrust::Untrusted
        );
    }

    #[test]
    fn dynamic_candidate_hooks_are_reported_as_runtime_disabled() {
        let temp = tempfile::tempdir().expect("tempdir");
        fs::write(
            temp.path().join("plugin.toml"),
            r#"
api_version = "1"
id = "dynamic.example"
name = "Dynamic Example"
version = "1.0.0"
capabilities = ["dynamic-candidates"]

[[hooks.dynamic_candidates]]
command = "cleanr-dynamic-example"
"#,
        )
        .expect("write manifest");
        let mut config = Config::default();
        config.plugins.dirs = vec![temp.path().to_path_buf()];

        let registry = RuleRegistry::load(&config).expect("load registry");

        assert!(registry.diagnostics().iter().any(|diagnostic| {
            diagnostic.code == "dynamic-candidates-runtime-disabled"
                && diagnostic.message.contains("dynamic.example")
        }));
    }

    fn test_rule_pack(id: &str, name: &str) -> String {
        format!(
            r#"
id = "{id}"
name = "{name}"
version = "1.0.0"
description = "Test"
categories = ["build-cache"]

[[rules]]
id = "target"
label = "Target"
category = "build-cache"
match = {{ dir_name = "target" }}
confidence = "high"
default_selected = true
action = "trash"
reason = "generated"
risk_note = "rebuild"
"#
        )
    }
}
