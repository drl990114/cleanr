#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use cleanr_config::AgentBackend;
use cleanr_core::{GlobalScanKind, ScanRequest};
use serde::{Deserialize, Serialize};

#[cfg(any(feature = "openai", feature = "ollama"))]
pub mod llm;

/// Create an agent provider from the configuration.
///
/// - `"local"` uses the built-in keyword matcher.
/// - `"openai"` / `"ollama"` use the configured LLM endpoint when the
///   corresponding crate feature is enabled.
pub fn create_agent(config: &cleanr_config::AgentConfig) -> Result<Box<dyn AgentProvider + Send>> {
    match config.provider {
        AgentBackend::Local => Ok(Box::new(LocalAgent)),
        #[cfg(any(feature = "openai", feature = "ollama"))]
        _ => Ok(Box::new(llm::LlmAgent::from_config(config)?)),
        #[cfg(not(any(feature = "openai", feature = "ollama")))]
        other => bail!("unsupported agent provider: {other}"),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionRequest {
    Scan(ScanRequest),
    Review,
    Plan,
    Clean { intent: CleanupIntent },
    Restore,
    Rules,
    Plugins,
    Languages,
    Tasks,
    Usage(ScanRequest),
    ExportPlan(Option<PathBuf>),
    Help,
    Quit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CleanupIntent {
    AgentRequest,
    UserRequest,
    ExplicitUserConfirmation,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub description_key: &'static str,
    pub requires_scan: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentResponse {
    pub message: String,
    pub actions: Vec<ActionRequest>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PathInsight {
    pub item_type: String,
    pub source: String,
    pub meaning: String,
    pub referenced_by: Vec<String>,
    pub risk: String,
    pub advice: String,
}

#[derive(Debug, Clone, Default)]
pub struct PathContext {
    pub size_bytes: u64,
    pub parent_path: Option<PathBuf>,
    pub rule_id: Option<String>,
    pub reason: Option<String>,
}

pub trait AgentProvider: Send {
    fn interpret(&self, input: &str) -> Result<AgentResponse>;
    fn explain_path(&self, path: &Path, context: &PathContext) -> Result<PathInsight>;
}

#[derive(Debug, Default)]
pub struct LocalAgent;

impl AgentProvider for LocalAgent {
    fn interpret(&self, input: &str) -> Result<AgentResponse> {
        let trimmed = input.trim();
        if trimmed.starts_with('/') {
            let action = parse_slash_command(trimmed)?;
            return Ok(AgentResponse {
                message: format!("queued {}", command_name(&action)),
                actions: vec![action],
            });
        }

        let lowered = trimmed.to_lowercase();
        if lowered.contains("cache")
            || lowered.contains("缓存")
            || lowered.contains("clean")
            || lowered.contains("清理")
        {
            return Ok(AgentResponse {
                message: "I'll scan the current root and open the review plan.".to_string(),
                actions: vec![
                    ActionRequest::Scan(ScanRequest::default()),
                    ActionRequest::Review,
                ],
            });
        }

        Ok(AgentResponse {
            message: "Try /scan, /review, /plan, or /help.".to_string(),
            actions: vec![ActionRequest::Help],
        })
    }

    fn explain_path(&self, path: &Path, _context: &PathContext) -> Result<PathInsight> {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let parent = path.parent().and_then(|p| p.to_str()).unwrap_or("");

        // Match well-known cache/build directories first.
        match name {
            "node_modules" => Ok(PathInsight {
                item_type: "dependency directory".into(),
                source: "npm / yarn / pnpm".into(),
                meaning: "JavaScript packages downloaded for the parent project. The project can reinstall them from package.json.".into(),
                referenced_by: vec![parent.into()],
                risk: "low".into(),
                advice: "Safe to remove if the project is not currently being developed.".into(),
            }),
            "target" => Ok(PathInsight {
                item_type: "build output directory".into(),
                source: "Cargo / Rust".into(),
                meaning: "Compiled Rust artifacts, intermediate files and dependency builds.".into(),
                referenced_by: vec![parent.into()],
                risk: "low".into(),
                advice: "Can be rebuilt with `cargo build`. Safe to delete.".into(),
            }),
            ".next" => Ok(PathInsight {
                item_type: "framework build output".into(),
                source: "Next.js".into(),
                meaning: "Generated static and server bundles for a Next.js application.".into(),
                referenced_by: vec![parent.into()],
                risk: "low".into(),
                advice: "Recreated by `next build`. Safe to delete.".into(),
            }),
            "DerivedData" => Ok(PathInsight {
                item_type: "Xcode build cache".into(),
                source: "Xcode".into(),
                meaning: "Compiled indexes, build products and debug symbols for Xcode projects.".into(),
                referenced_by: vec!["Xcode projects".into()],
                risk: "low".into(),
                advice: "Xcode will rebuild it automatically. Safe to clean.".into(),
            }),
            ".gradle" | "build" => Ok(PathInsight {
                item_type: "Gradle build output".into(),
                source: "Gradle / Android".into(),
                meaning: "Compiled Java/Kotlin/Android classes, resources and intermediate files.".into(),
                referenced_by: vec![parent.into()],
                risk: "low".into(),
                advice: "Recreated on the next build. Safe to delete.".into(),
            }),
            "__pycache__" | ".pytest_cache" | ".mypy_cache" => Ok(PathInsight {
                item_type: "Python cache".into(),
                source: "Python tooling".into(),
                meaning: "Bytecode or type-check cache generated by Python tools.".into(),
                referenced_by: vec![parent.into()],
                risk: "low".into(),
                advice: "Automatically regenerated. Safe to delete.".into(),
            }),
            ".git" => Ok(PathInsight {
                item_type: "Git repository metadata".into(),
                source: "Git".into(),
                meaning: "Object database, refs and configuration for a Git repository.".into(),
                referenced_by: vec!["git commands".into()],
                risk: "high".into(),
                advice: "Do not delete unless you intend to remove version control history.".into(),
            }),
            _ => {
                // Fallback heuristic based on path components.
                let path_str = path.to_string_lossy().to_lowercase();
                if path_str.contains("cache") {
                    Ok(PathInsight {
                        item_type: "cache directory".into(),
                        source: "unknown".into(),
                        meaning: "Likely an application cache that can often be rebuilt.".into(),
                        referenced_by: vec!["unknown".into()],
                        risk: "medium".into(),
                        advice: "Review the application before deleting.".into(),
                    })
                } else if path_str.contains("log") {
                    Ok(PathInsight {
                        item_type: "log files".into(),
                        source: "unknown".into(),
                        meaning: "Application or system log output.".into(),
                        referenced_by: vec!["system / applications".into()],
                        risk: "low".into(),
                        advice: "Old logs are usually safe to remove if no debugging is needed.".into(),
                    })
                } else {
                    Ok(PathInsight {
                        item_type: "file or directory".into(),
                        source: "unknown".into(),
                        meaning: "No specific knowledge about this path.".into(),
                        referenced_by: vec![],
                        risk: "medium".into(),
                        advice: "Inspect contents manually before cleaning.".into(),
                    })
                }
            }
        }
    }
}

pub fn parse_slash_command(input: &str) -> Result<ActionRequest> {
    let mut parts = split_command(input)?;
    if parts.is_empty() {
        bail!("empty command");
    }
    let command = parts.remove(0);
    let args = parts;

    match command.as_str() {
        "/scan" => Ok(ActionRequest::Scan(parse_scan_request(args)?)),
        "/review" => Ok(ActionRequest::Review),
        "/plan" => Ok(ActionRequest::Plan),
        "/clean" => Ok(ActionRequest::Clean {
            intent: if args.iter().any(|arg| arg == "--confirm") {
                CleanupIntent::ExplicitUserConfirmation
            } else {
                CleanupIntent::UserRequest
            },
        }),
        "/restore" => Ok(ActionRequest::Restore),
        "/rules" => Ok(ActionRequest::Rules),
        "/plugins" => Ok(ActionRequest::Plugins),
        "/languages" | "/lang" => Ok(ActionRequest::Languages),
        "/tasks" => Ok(ActionRequest::Tasks),
        "/usage" | "/stats" => Ok(ActionRequest::Usage(parse_scan_request(args)?)),
        "/export-plan" => Ok(ActionRequest::ExportPlan(args.first().map(PathBuf::from))),
        "/help" => Ok(ActionRequest::Help),
        "/quit" | "/q" => Ok(ActionRequest::Quit),
        other => bail!("unknown command: {other}"),
    }
}

fn parse_scan_request(args: Vec<String>) -> Result<ScanRequest> {
    let mut request = ScanRequest::default();
    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        if arg == "--global" {
            request.include_global = true;
        } else if arg == "--global-kind" {
            index += 1;
            let Some(kind) = args.get(index) else {
                bail!("--global-kind requires a value");
            };
            request.include_global = true;
            request.global_kinds.push(parse_global_kind(kind)?);
        } else if let Some(kind) = arg.strip_prefix("--global-kind=") {
            request.include_global = true;
            request.global_kinds.push(parse_global_kind(kind)?);
        } else {
            request.paths.push(PathBuf::from(arg));
        }
        index += 1;
    }
    request.global_kinds.sort();
    request.global_kinds.dedup();
    Ok(request)
}

fn parse_global_kind(value: &str) -> Result<GlobalScanKind> {
    value.parse().map_err(anyhow::Error::msg)
}

fn split_command(input: &str) -> Result<Vec<String>> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;

    for ch in input.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        match quote {
            Some(active) if ch == active => quote = None,
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    parts.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }

    if escaped {
        current.push('\\');
    }
    if let Some(active) = quote {
        bail!("unterminated {active} quote");
    }
    if !current.is_empty() {
        parts.push(current);
    }
    Ok(parts)
}

#[must_use]
pub fn command_palette(has_scan_results: bool) -> Vec<CommandInfo> {
    vec![
        CommandInfo {
            name: "/scan [path...]",
            description: "Scan current or specified roots",
            description_key: "command_scan",
            requires_scan: false,
        },
        CommandInfo {
            name: "/scan --global",
            description: "Scan all known system cleanup locations",
            description_key: "command_scan_global",
            requires_scan: false,
        },
        CommandInfo {
            name: "/usage [path...]",
            description: "Scan current or specified roots and show usage",
            description_key: "command_usage",
            requires_scan: false,
        },
        CommandInfo {
            name: "/usage --global",
            description: "Scan known system cleanup locations and show usage",
            description_key: "command_usage_global",
            requires_scan: false,
        },
        CommandInfo {
            name: "/review",
            description: "Build and show cleanup candidates",
            description_key: "command_review",
            requires_scan: true,
        },
        CommandInfo {
            name: "/plan",
            description: "Generate an AI-readable JSON cleanup plan",
            description_key: "command_plan",
            requires_scan: true,
        },
        CommandInfo {
            name: "/clean",
            description: "Preview the selected cleanup plan",
            description_key: "command_clean",
            requires_scan: true,
        },
        CommandInfo {
            name: "/clean --confirm",
            description: "Move selected items to the system trash",
            description_key: "command_clean_confirm",
            requires_scan: true,
        },
        CommandInfo {
            name: "/export-plan [path]",
            description: "Write the current JSON plan to disk",
            description_key: "command_export_plan",
            requires_scan: true,
        },
        CommandInfo {
            name: "/restore",
            description: "Restore items from a cleanup run",
            description_key: "command_restore",
            requires_scan: false,
        },
        CommandInfo {
            name: "/rules",
            description: "Show active cleanup rules",
            description_key: "command_rules",
            requires_scan: false,
        },
        CommandInfo {
            name: "/plugins",
            description: "Show loaded declaration-only plugins",
            description_key: "command_plugins",
            requires_scan: false,
        },
        CommandInfo {
            name: "/languages",
            description: "Show loaded language packs",
            description_key: "command_languages",
            requires_scan: false,
        },
        CommandInfo {
            name: "/tasks",
            description: "Show task history",
            description_key: "command_tasks",
            requires_scan: false,
        },
        CommandInfo {
            name: "/help",
            description: "Show command help",
            description_key: "command_help",
            requires_scan: false,
        },
        CommandInfo {
            name: "/quit",
            description: "Quit cleanr",
            description_key: "command_quit",
            requires_scan: false,
        },
    ]
    .into_iter()
    .filter(|cmd| !cmd.requires_scan || has_scan_results)
    .collect()
}

pub(crate) fn command_name(action: &ActionRequest) -> &'static str {
    match action {
        ActionRequest::Scan(_) => "/scan",
        ActionRequest::Review => "/review",
        ActionRequest::Plan => "/plan",
        ActionRequest::Clean { .. } => "/clean",
        ActionRequest::Restore => "/restore",
        ActionRequest::Rules => "/rules",
        ActionRequest::Plugins => "/plugins",
        ActionRequest::Languages => "/languages",
        ActionRequest::Tasks => "/tasks",
        ActionRequest::Usage(_) => "/usage",
        ActionRequest::ExportPlan(_) => "/export-plan",
        ActionRequest::Help => "/help",
        ActionRequest::Quit => "/quit",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_scan_command_with_paths() {
        let action = parse_slash_command("/scan . /tmp").expect("parse");
        assert_eq!(
            action,
            ActionRequest::Scan(ScanRequest::paths(vec![
                PathBuf::from("."),
                PathBuf::from("/tmp")
            ]))
        );
    }

    #[test]
    fn parses_quoted_slash_command_arguments() {
        let action =
            parse_slash_command(r#"/scan "/tmp/project with spaces" '/tmp/other path' --global"#)
                .expect("parse");
        assert_eq!(
            action,
            ActionRequest::Scan(ScanRequest {
                paths: vec![
                    PathBuf::from("/tmp/project with spaces"),
                    PathBuf::from("/tmp/other path"),
                ],
                include_global: true,
                global_kinds: Vec::new(),
            })
        );

        assert_eq!(
            parse_slash_command(r#"/export-plan "plans/cleanr plan.json""#).expect("parse"),
            ActionRequest::ExportPlan(Some(PathBuf::from("plans/cleanr plan.json")))
        );
        assert!(parse_slash_command(r#"/scan "unterminated"#).is_err());
    }

    #[test]
    fn parses_global_scan_kinds() {
        assert_eq!(
            parse_slash_command("/scan --global-kind browser-caches --global-kind logs")
                .expect("parse"),
            ActionRequest::Scan(ScanRequest {
                paths: Vec::new(),
                include_global: true,
                global_kinds: vec![GlobalScanKind::BrowserCaches, GlobalScanKind::Logs],
            })
        );
        assert!(parse_slash_command("/scan --global-kind unknown").is_err());
    }

    #[test]
    fn parses_clean_confirm_as_explicit_safety_gate() {
        let action = parse_slash_command("/clean --confirm").expect("parse");
        assert_eq!(
            action,
            ActionRequest::Clean {
                intent: CleanupIntent::ExplicitUserConfirmation
            }
        );
    }

    #[test]
    fn parses_usage_command_aliases() {
        assert_eq!(
            parse_slash_command("/usage").expect("parse"),
            ActionRequest::Usage(ScanRequest::default())
        );
        assert_eq!(
            parse_slash_command("/stats").expect("parse"),
            ActionRequest::Usage(ScanRequest::default())
        );
        assert_eq!(
            parse_slash_command("/usage /tmp .").expect("parse"),
            ActionRequest::Usage(ScanRequest::paths(vec![
                PathBuf::from("/tmp"),
                PathBuf::from(".")
            ]))
        );
    }

    #[test]
    fn local_agent_maps_plain_language_to_scan_review() {
        let response = LocalAgent
            .interpret("帮我找一下可以清理的缓存")
            .expect("interpret");
        assert_eq!(response.actions.len(), 2);
        assert!(matches!(response.actions[0], ActionRequest::Scan(_)));
        assert_eq!(response.actions[1], ActionRequest::Review);
    }

    #[test]
    fn unknown_slash_command_is_reported_and_plain_text_falls_back_to_help() {
        assert!(parse_slash_command("/does-not-exist").is_err());
        let response = LocalAgent.interpret("hello there").expect("interpret");
        assert_eq!(response.actions, vec![ActionRequest::Help]);
    }

    #[test]
    fn command_palette_hides_scan_dependent_commands_until_results_exist() {
        let before_scan = command_palette(false);
        assert!(before_scan.iter().all(|command| !command.requires_scan));
        assert!(!before_scan.iter().any(|command| command.name == "/review"));
        assert!(before_scan.iter().any(|command| {
            command.name == "/scan --global" && command.description_key == "command_scan_global"
        }));
        assert!(before_scan.iter().any(|command| {
            command.name == "/usage --global" && command.description_key == "command_usage_global"
        }));

        let after_scan = command_palette(true);
        assert!(after_scan.iter().any(|command| command.name == "/review"));
        assert!(
            after_scan
                .iter()
                .any(|command| command.name == "/clean --confirm")
        );
    }

    #[test]
    fn local_path_explanations_distinguish_known_and_unknown_paths() {
        let known = LocalAgent
            .explain_path(Path::new("/repo/node_modules"), &PathContext::default())
            .expect("known path");
        assert_eq!(known.source, "npm / yarn / pnpm");
        assert_eq!(known.risk, "low");

        let unknown = LocalAgent
            .explain_path(Path::new("/repo/user-data"), &PathContext::default())
            .expect("unknown path");
        assert_eq!(unknown.source, "unknown");
        assert_eq!(unknown.risk, "medium");
    }
}
