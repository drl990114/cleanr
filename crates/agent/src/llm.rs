#![cfg(any(feature = "openai", feature = "ollama"))]

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use cleanr_config::AgentBackend;
use serde::{Deserialize, Serialize};

use crate::{ActionRequest, AgentProvider, AgentResponse, PathContext, PathInsight, command_name};

const SYSTEM_PROMPT: &str = r#"You are the command parser for cleanr, a safe local disk cleanup assistant.

Available actions (respond with a JSON array of these objects):
- {"action": "scan", "paths": ["."]}              - scan one or more roots for cleanup candidates; use ["--global"] for known developer caches
- {"action": "review"}                               - build and show the cleanup review plan
- {"action": "plan"}                                 - export an AI-readable JSON cleanup plan
- {"action": "clean"}                                - request cleanup; the host must ask the user for confirmation
- {"action": "restore"}                              - open cleanup runs so the user can choose one to restore
- {"action": "rules"}                                - show active cleanup rules
- {"action": "plugins"}                              - show loaded plugins
- {"action": "languages"}                            - show loaded language packs
- {"action": "tasks"}                                - show task history
- {"action": "usage", "paths": ["."]}               - scan roots and show disk usage summary
- {"action": "export_plan", "path": "plan.json"}    - write the current JSON plan to disk
- {"action": "help"}                                 - show command help
- {"action": "quit"}                                 - quit cleanr

Rules:
1. Output ONLY a JSON array. Do not include explanations, markdown, or commentary outside the JSON.
2. An empty "paths" array means "use the current directory / the user's default roots".
3. You cannot authorize cleanup. A "clean" action only asks the host to show a local confirmation.
4. If the request is ambiguous, return [{"action": "help"}].
"#;

const EXPLAIN_SYSTEM_PROMPT: &str = r#"You are a disk cleanup assistant. Explain a file or directory so the user can decide whether to clean it.

Respond with ONLY a single JSON object matching this schema:
{
  "item_type": "short category, e.g. dependency directory / build output / log files / user data",
  "source": "the tool or ecosystem that created it, e.g. npm / Xcode / Docker / unknown",
  "meaning": "1-2 sentences describing what this path contains and how it is used",
  "referenced_by": ["list of projects, processes or contexts that reference this path"],
  "risk": "low | medium | high",
  "advice": "clear, actionable recommendation for the user"
}

Rules:
1. Output ONLY the JSON object. No markdown fences, no commentary.
2. Be conservative: if unsure, set risk to "high" and advise manual inspection.
3. For well-known caches (node_modules, target, DerivedData, .next, __pycache__), state that they can be rebuilt.
"#;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
enum LlmAction {
    Scan { paths: Vec<String> },
    Review,
    Plan,
    Clean,
    Restore,
    Rules,
    Plugins,
    Languages,
    Tasks,
    Usage { paths: Vec<String> },
    ExportPlan { path: Option<String> },
    Help,
    Quit,
}

impl LlmAction {
    fn into_action_request(self) -> ActionRequest {
        match self {
            LlmAction::Scan { paths } => {
                ActionRequest::Scan(paths.into_iter().map(PathBuf::from).collect())
            }
            LlmAction::Review => ActionRequest::Review,
            LlmAction::Plan => ActionRequest::Plan,
            LlmAction::Clean => ActionRequest::Clean {
                intent: crate::CleanupIntent::AgentRequest,
            },
            LlmAction::Restore => ActionRequest::Restore,
            LlmAction::Rules => ActionRequest::Rules,
            LlmAction::Plugins => ActionRequest::Plugins,
            LlmAction::Languages => ActionRequest::Languages,
            LlmAction::Tasks => ActionRequest::Tasks,
            LlmAction::Usage { paths } => {
                ActionRequest::Usage(paths.into_iter().map(PathBuf::from).collect())
            }
            LlmAction::ExportPlan { path } => ActionRequest::ExportPlan(path.map(PathBuf::from)),
            LlmAction::Help => ActionRequest::Help,
            LlmAction::Quit => ActionRequest::Quit,
        }
    }
}

enum LlmClientKind {
    #[cfg(feature = "openai")]
    OpenAi(OpenAiClient),
    #[cfg(feature = "ollama")]
    Ollama(OllamaClient),
}

impl LlmClientKind {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        match self {
            #[cfg(feature = "openai")]
            LlmClientKind::OpenAi(c) => c.complete(system, user).await,
            #[cfg(feature = "ollama")]
            LlmClientKind::Ollama(c) => c.complete(system, user).await,
        }
    }
}

pub struct LlmAgent {
    client: LlmClientKind,
    model: String,
    runtime: tokio::runtime::Runtime,
}

impl LlmAgent {
    pub fn from_config(config: &cleanr_config::AgentConfig) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .context("failed to create tokio runtime for LLM agent")?;

        match config.provider {
            #[cfg(feature = "openai")]
            AgentBackend::Openai => {
                let model = config
                    .model
                    .clone()
                    .unwrap_or_else(|| "gpt-4o-mini".to_string());
                let api_key = resolve_api_key(config)?;
                let mut openai_config =
                    async_openai::config::OpenAIConfig::new().with_api_key(api_key);
                if let Some(endpoint) = &config.endpoint {
                    openai_config = openai_config.with_api_base(endpoint.clone());
                }
                let client = async_openai::Client::with_config(openai_config);
                Ok(Self {
                    client: LlmClientKind::OpenAi(OpenAiClient {
                        client,
                        model: model.clone(),
                    }),
                    model,
                    runtime,
                })
            }
            #[cfg(feature = "ollama")]
            AgentBackend::Ollama => {
                let model = config
                    .model
                    .clone()
                    .unwrap_or_else(|| "llama3.2".to_string());
                let ollama = build_ollama(config)?;
                Ok(Self {
                    client: LlmClientKind::Ollama(OllamaClient {
                        ollama,
                        model: model.clone(),
                    }),
                    model,
                    runtime,
                })
            }
            other => bail!("unsupported LLM provider: {other}"),
        }
    }
}

impl AgentProvider for LlmAgent {
    fn interpret(&self, input: &str) -> Result<AgentResponse> {
        let user_prompt = build_user_prompt(input);
        let raw = self
            .runtime
            .block_on(self.client.complete(SYSTEM_PROMPT, &user_prompt))?;
        let actions = parse_actions(&raw)?;
        let message = summarize(&actions, &self.model);
        Ok(AgentResponse { message, actions })
    }

    fn explain_path(&self, path: &std::path::Path, context: &PathContext) -> Result<PathInsight> {
        let user_prompt = build_explain_prompt(path, context);
        let raw = self
            .runtime
            .block_on(self.client.complete(EXPLAIN_SYSTEM_PROMPT, &user_prompt))?;
        parse_insight(&raw)
    }
}

#[cfg(feature = "openai")]
struct OpenAiClient {
    client: async_openai::Client<async_openai::config::OpenAIConfig>,
    model: String,
}

#[cfg(feature = "openai")]
impl OpenAiClient {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        use async_openai::types::chat::{
            ChatCompletionRequestSystemMessage, ChatCompletionRequestUserMessage,
            CreateChatCompletionRequestArgs,
        };

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages([
                ChatCompletionRequestSystemMessage::from(system).into(),
                ChatCompletionRequestUserMessage::from(user).into(),
            ])
            .max_completion_tokens(512u32)
            .build()?;

        let response = self.client.chat().create(request).await?;
        let content = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();
        Ok(content.trim().to_string())
    }
}

#[cfg(feature = "ollama")]
struct OllamaClient {
    ollama: ollama_rs::Ollama,
    model: String,
}

#[cfg(feature = "ollama")]
impl OllamaClient {
    async fn complete(&self, system: &str, user: &str) -> Result<String> {
        use ollama_rs::generation::chat::request::ChatMessageRequest;
        use ollama_rs::generation::chat::{ChatMessage, MessageRole};

        let request = ChatMessageRequest::new(
            self.model.clone(),
            vec![
                ChatMessage::new(MessageRole::System, system.to_string()),
                ChatMessage::new(MessageRole::User, user.to_string()),
            ],
        );

        let response = self.ollama.send_chat_messages(request).await?;
        Ok(response.message.content.trim().to_string())
    }
}

fn resolve_api_key(config: &cleanr_config::AgentConfig) -> Result<String> {
    std::env::var(&config.api_key_env).with_context(|| {
        format!(
            "missing API key in environment variable {}",
            config.api_key_env
        )
    })
}

#[cfg(feature = "ollama")]
fn build_ollama(config: &cleanr_config::AgentConfig) -> Result<ollama_rs::Ollama> {
    let Some(endpoint) = &config.endpoint else {
        return Ok(ollama_rs::Ollama::default());
    };

    let url = url::Url::parse(endpoint)
        .with_context(|| format!("invalid ollama endpoint: {endpoint}"))?;
    let host = format!(
        "{}://{}",
        url.scheme(),
        url.host_str().unwrap_or("localhost")
    );
    let port = url.port().unwrap_or(11434);

    Ok(ollama_rs::Ollama::builder().host(host).port(port).build())
}

fn build_user_prompt(input: &str) -> String {
    format!(
        "User request: \"{}\"\n\nRespond with a JSON array of actions. Empty paths mean use the current directory.",
        input
    )
}

fn build_explain_prompt(path: &std::path::Path, context: &PathContext) -> String {
    let path_display = path.display();
    let parent = context
        .parent_path
        .as_ref()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let rule_id = context.rule_id.as_deref().unwrap_or("unknown");
    let reason = context.reason.as_deref().unwrap_or("unknown");
    format!(
        "Path: {path_display}\nParent: {parent}\nSize: {size} bytes\nMatched rule: {rule_id}\nRule reason: {reason}\n\nExplain this path.",
        size = context.size_bytes,
    )
}

fn parse_actions(raw: &str) -> Result<Vec<ActionRequest>> {
    let cleaned = strip_markdown_fences(raw).trim();
    let actions: Vec<LlmAction> =
        serde_json::from_str(cleaned).context("LLM returned invalid JSON action list")?;
    Ok(actions
        .into_iter()
        .map(LlmAction::into_action_request)
        .collect())
}

fn parse_insight(raw: &str) -> Result<PathInsight> {
    let cleaned = strip_markdown_fences(raw).trim();
    serde_json::from_str(cleaned).context("LLM returned invalid insight JSON")
}

fn strip_markdown_fences(s: &str) -> &str {
    let trimmed = s.trim();
    if let Some(inner) = trimmed.strip_prefix("```json") {
        inner.trim_end_matches("```").trim()
    } else if let Some(inner) = trimmed.strip_prefix("```") {
        inner.trim_end_matches("```").trim()
    } else {
        trimmed
    }
}

fn summarize(actions: &[ActionRequest], model: &str) -> String {
    if actions.is_empty() {
        return format!("No action recognized (model: {model}).");
    }
    let names: Vec<_> = actions.iter().map(command_name).collect();
    format!("{} (model: {model})", names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json_action_list() {
        let raw = r#"[{"action": "scan", "paths": ["."]}, {"action": "review"}]"#;
        let actions = parse_actions(raw).expect("parse");
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], ActionRequest::Scan(_)));
        assert_eq!(actions[1], ActionRequest::Review);
    }

    #[test]
    fn strips_markdown_fence() {
        let raw = "```json\n[{\"action\": \"help\"}]\n```";
        let actions = parse_actions(raw).expect("parse");
        assert_eq!(actions, vec![ActionRequest::Help]);
    }

    #[test]
    fn returns_help_for_unknown_action() {
        let raw = r#"[{"action": "unknown_thing"}]"#;
        assert!(parse_actions(raw).is_err());
    }
}
