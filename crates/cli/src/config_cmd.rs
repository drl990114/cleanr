#![forbid(unsafe_code)]

use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use cleanr_config::{AgentBackend, CleanupAction, Config, UiTheme};

fn resolve_config_path(path: Option<PathBuf>) -> Result<PathBuf> {
    path.or_else(cleanr_config::default_config_path)
        .context("platform config directory is unavailable; pass --config")
}

pub fn path(config_path: Option<PathBuf>) -> Result<()> {
    let path = resolve_config_path(config_path)?;
    println!("{}", path.display());
    Ok(())
}

pub fn init(config_path: Option<PathBuf>, force: bool) -> Result<()> {
    let path = resolve_config_path(config_path)?;

    if path.exists() && !force {
        println!("Config already exists at {}", path.display());
        println!("Use --force to overwrite with defaults.");
        return Ok(());
    }

    let config = Config::default();
    config.save_to(&path)?;
    println!("Config written to {}", path.display());
    Ok(())
}

pub fn get(config_path: Option<PathBuf>, key: &str) -> Result<()> {
    let path = resolve_config_path(config_path)?;
    let config = if path.exists() {
        Config::load_from(&path)?
    } else {
        Config::default()
    };

    println!("{}", get_value(&config, key)?);
    Ok(())
}

pub fn set(config_path: Option<PathBuf>, key: &str, value: &str) -> Result<()> {
    let path = resolve_config_path(config_path)?;
    let mut config = if path.exists() {
        Config::load_from(&path)?
    } else {
        Config::default()
    };

    apply_value(&mut config, key, value)?;
    config.save_to(&path)?;
    println!("Set {key} = {value} in {}", path.display());
    Ok(())
}

pub fn set_agent(
    config_path: Option<PathBuf>,
    provider: Option<String>,
    model: Option<String>,
    endpoint: Option<String>,
    api_key_env: Option<String>,
) -> Result<()> {
    if provider.is_none() && model.is_none() && endpoint.is_none() && api_key_env.is_none() {
        bail!("at least one agent field must be provided");
    }

    let path = resolve_config_path(config_path)?;
    let mut config = if path.exists() {
        Config::load_from(&path)?
    } else {
        Config::default()
    };

    if let Some(v) = provider {
        config.agent.provider = v.parse()?;
    }
    if let Some(v) = model {
        config.agent.model = normalize_opt_string(v);
    }
    if let Some(v) = endpoint {
        config.agent.endpoint = normalize_opt_string(v);
    }
    if let Some(v) = api_key_env {
        config.agent.api_key_env = v;
    }

    config.save_to(&path)?;
    println!("Agent config updated in {}", path.display());
    Ok(())
}

fn normalize_opt_string(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

fn get_value(config: &Config, key: &str) -> Result<String> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["agent", "provider"] => Ok(config.agent.provider.to_string()),
        ["agent", "model"] => Ok(config.agent.model.clone().unwrap_or_default()),
        ["agent", "endpoint"] => Ok(config.agent.endpoint.clone().unwrap_or_default()),
        ["agent", "api_key_env"] => Ok(config.agent.api_key_env.clone()),
        ["scan", "stay_on_filesystem"] => Ok(config.scan.stay_on_filesystem.to_string()),
        ["cleanup", "default_action"] => Ok(config.cleanup.default_action.to_string()),
        ["cleanup", "require_confirm"] => Ok(config.cleanup.require_confirm.to_string()),
        ["i18n", "locale"] => Ok(config.i18n.locale.clone().unwrap_or_default()),
        ["ui", "theme"] => Ok(config.ui.theme.to_string()),
        _ => bail!("unknown config key: {key}"),
    }
}

fn apply_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    let parts: Vec<&str> = key.split('.').collect();
    match parts.as_slice() {
        ["agent", "provider"] => config.agent.provider = value.parse::<AgentBackend>()?,
        ["agent", "model"] => config.agent.model = normalize_opt_string(value.to_string()),
        ["agent", "endpoint"] => config.agent.endpoint = normalize_opt_string(value.to_string()),
        ["agent", "api_key_env"] => config.agent.api_key_env = value.to_string(),
        ["scan", "stay_on_filesystem"] => {
            config.scan.stay_on_filesystem = parse_bool(value)?;
        }
        ["cleanup", "default_action"] => {
            config.cleanup.default_action = value.parse::<CleanupAction>()?;
        }
        ["cleanup", "require_confirm"] => {
            config.cleanup.require_confirm = parse_bool(value)?;
        }
        ["i18n", "locale"] => config.i18n.locale = normalize_opt_string(value.to_string()),
        ["ui", "theme"] => config.ui.theme = value.parse::<UiTheme>()?,
        _ => bail!("unknown config key: {key}"),
    }
    Ok(())
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" => Ok(true),
        "false" | "0" | "no" | "off" => Ok(false),
        _ => bail!("expected boolean, got: {value}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boolean_parser_accepts_cli_friendly_spellings() {
        for value in ["true", "TRUE", "1", "yes", "on"] {
            assert!(parse_bool(value).expect(value));
        }
        for value in ["false", "FALSE", "0", "no", "off"] {
            assert!(!parse_bool(value).expect(value));
        }
        assert!(parse_bool("maybe").is_err());
    }

    #[test]
    fn supported_values_round_trip_through_get_and_apply() {
        let mut config = Config::default();
        for (key, value, expected) in [
            ("agent.provider", "ollama", "ollama"),
            ("agent.model", "llama3.2", "llama3.2"),
            (
                "agent.endpoint",
                "http://localhost:11434",
                "http://localhost:11434",
            ),
            ("agent.api_key_env", "OLLAMA_KEY", "OLLAMA_KEY"),
            ("scan.stay_on_filesystem", "yes", "true"),
            ("cleanup.default_action", "trash", "trash"),
            ("cleanup.require_confirm", "off", "false"),
            ("i18n.locale", "zh-CN", "zh-CN"),
            ("ui.theme", "dark", "dark"),
        ] {
            apply_value(&mut config, key, value).expect(key);
            assert_eq!(get_value(&config, key).expect(key), expected);
        }

        assert!(apply_value(&mut config, "missing.key", "value").is_err());
        assert!(get_value(&config, "missing.key").is_err());
    }

    #[test]
    fn empty_optional_strings_clear_agent_and_locale_values() {
        let mut config = Config::default();
        config.agent.model = Some("model".to_string());
        config.agent.endpoint = Some("endpoint".to_string());
        config.i18n.locale = Some("zh-CN".to_string());

        for key in ["agent.model", "agent.endpoint", "i18n.locale"] {
            apply_value(&mut config, key, "").expect(key);
            assert_eq!(get_value(&config, key).expect(key), "");
        }
    }

    #[test]
    fn set_and_set_agent_persist_configuration() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("nested").join("config.toml");

        set(Some(path.clone()), "ui.theme", "light").expect("set theme");
        set_agent(
            Some(path.clone()),
            Some("openai".to_string()),
            Some("gpt-test".to_string()),
            Some("https://example.invalid/v1".to_string()),
            Some("OPENAI_TEST_KEY".to_string()),
        )
        .expect("set agent");

        let config = Config::load_from(path).expect("load config");
        assert_eq!(config.ui.theme, UiTheme::Light);
        assert_eq!(config.agent.provider, AgentBackend::Openai);
        assert_eq!(config.agent.model.as_deref(), Some("gpt-test"));
        assert_eq!(config.agent.api_key_env, "OPENAI_TEST_KEY");
    }

    #[test]
    fn invalid_set_does_not_create_or_replace_configuration() {
        let temp = tempfile::tempdir().expect("tempdir");
        let missing = temp.path().join("missing.toml");
        assert!(set(Some(missing.clone()), "ui.theme", "neon").is_err());
        assert!(!missing.exists());

        let existing = temp.path().join("existing.toml");
        Config::default().save_to(&existing).expect("seed config");
        let before = std::fs::read_to_string(&existing).expect("before");
        assert!(set(Some(existing.clone()), "agent.api_key_env", "").is_err());
        assert_eq!(std::fs::read_to_string(existing).expect("after"), before);
    }

    #[test]
    fn set_agent_requires_at_least_one_field() {
        let error = set_agent(None, None, None, None, None).expect_err("missing fields");
        assert!(error.to_string().contains("at least one"));
    }
}
