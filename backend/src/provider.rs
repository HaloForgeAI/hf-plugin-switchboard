use crate::fs_util::{
    atomic_write, read_json_object_or_empty, read_optional_bytes, restore_optional_bytes,
    write_json_pretty,
};
use crate::paths::SwitchboardPaths;
use crate::types::{
    ApplyProviderArgs, BOTH_TARGET, CLAUDE_TARGET, CODEX_TARGET, DEFAULT_CODEX_PROVIDER_ID,
    DiscoverModelsArgs, DiscoverModelsResult, LEGACY_CODEX_PROVIDER_ID,
};
use hf_plugin_api::PluginError;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

pub fn validate_provider_args(args: &ApplyProviderArgs) -> Result<(), PluginError> {
    if !matches!(
        args.target.as_str(),
        CLAUDE_TARGET | CODEX_TARGET | BOTH_TARGET
    ) {
        return Err(PluginError::Custom(format!(
            "unsupported target '{}'",
            args.target
        )));
    }
    if args.name.trim().is_empty() {
        return Err(PluginError::Custom("provider name is required".into()));
    }
    if args.base_url.trim().is_empty() {
        return Err(PluginError::Custom("base URL is required".into()));
    }
    if args.api_key.trim().is_empty() {
        return Err(PluginError::Custom("API key is required".into()));
    }
    Ok(())
}

pub fn validate_models_args(args: &DiscoverModelsArgs) -> Result<(), PluginError> {
    if args.base_url.trim().is_empty() {
        return Err(PluginError::Custom("base URL is required".into()));
    }
    if args.api_key.trim().is_empty() {
        return Err(PluginError::Custom("API key is required".into()));
    }
    Ok(())
}

pub fn discover_models(args: &DiscoverModelsArgs) -> Result<DiscoverModelsResult, PluginError> {
    let url = build_models_url(&args.base_url, args.models_path.as_deref());
    let body = http_get_json(&url, args.api_key.trim())?;
    let root: Value = serde_json::from_str(&body).map_err(|error| {
        PluginError::Serialization(format!("models response is not valid JSON: {error}"))
    })?;
    let models = parse_model_ids(&root);
    Ok(DiscoverModelsResult { models })
}

pub fn provider_backup_paths(
    paths: &SwitchboardPaths,
    args: &ApplyProviderArgs,
) -> Vec<std::path::PathBuf> {
    let mut backup_paths = Vec::new();
    if applies_to(&args.target, CLAUDE_TARGET) {
        backup_paths.push(paths.claude_settings_path.clone());
        if args.set_claude_primary_api_key.unwrap_or(false) {
            backup_paths.push(paths.claude_config_path.clone());
        }
        if args.skip_claude_onboarding.unwrap_or(false) {
            backup_paths.push(paths.claude_mcp_path.clone());
        }
    }
    if applies_to(&args.target, CODEX_TARGET) {
        backup_paths.push(paths.codex_auth_path.clone());
        backup_paths.push(paths.codex_config_path.clone());
    }
    backup_paths
}

pub fn apply_provider(
    paths: &SwitchboardPaths,
    args: &ApplyProviderArgs,
) -> Result<Vec<std::path::PathBuf>, PluginError> {
    let mut touched = Vec::new();

    if applies_to(&args.target, CLAUDE_TARGET) {
        write_claude_provider(paths, args)?;
        touched.push(paths.claude_settings_path.clone());

        if args.set_claude_primary_api_key.unwrap_or(false) {
            set_claude_primary_api_key(&paths.claude_config_path)?;
            touched.push(paths.claude_config_path.clone());
        }

        if args.skip_claude_onboarding.unwrap_or(false) {
            set_claude_onboarding(&paths.claude_mcp_path)?;
            touched.push(paths.claude_mcp_path.clone());
        }
    }

    if applies_to(&args.target, CODEX_TARGET) {
        write_codex_provider(paths, args)?;
        touched.push(paths.codex_auth_path.clone());
        touched.push(paths.codex_config_path.clone());
    }

    Ok(touched)
}

pub fn applies_to(target: &str, app: &str) -> bool {
    target == app || target == BOTH_TARGET
}

fn write_claude_provider(
    paths: &SwitchboardPaths,
    args: &ApplyProviderArgs,
) -> Result<(), PluginError> {
    let mut root = read_json_object_or_empty(&paths.claude_settings_path)?;
    let object = root.as_object_mut().ok_or_else(|| {
        PluginError::Serialization("Claude settings root must be an object".into())
    })?;

    let env = object
        .entry("env")
        .or_insert_with(|| Value::Object(Map::new()));
    if !env.is_object() {
        *env = Value::Object(Map::new());
    }
    let env_obj = env.as_object_mut().ok_or_else(|| {
        PluginError::Serialization("Claude settings env must be an object".into())
    })?;

    let model = defaulted(args.model.as_deref(), "claude-sonnet-4-6");
    let haiku = defaulted(args.haiku_model.as_deref(), &model);
    let sonnet = defaulted(args.sonnet_model.as_deref(), &model);
    let opus = defaulted(args.opus_model.as_deref(), &model);

    env_obj.insert("ANTHROPIC_BASE_URL".into(), json!(args.base_url.trim()));
    env_obj.insert("ANTHROPIC_AUTH_TOKEN".into(), json!(args.api_key.trim()));
    env_obj.insert("ANTHROPIC_MODEL".into(), json!(model));
    env_obj.insert("ANTHROPIC_DEFAULT_HAIKU_MODEL".into(), json!(haiku));
    env_obj.insert("ANTHROPIC_DEFAULT_SONNET_MODEL".into(), json!(sonnet));
    env_obj.insert("ANTHROPIC_DEFAULT_OPUS_MODEL".into(), json!(opus));

    write_json_pretty(&paths.claude_settings_path, &root)
}

fn set_claude_primary_api_key(path: &Path) -> Result<(), PluginError> {
    let mut root = read_json_object_or_empty(path)?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| PluginError::Serialization("Claude config root must be an object".into()))?;
    object.insert("primaryApiKey".into(), json!("any"));
    write_json_pretty(path, &root)
}

fn set_claude_onboarding(path: &Path) -> Result<(), PluginError> {
    let mut root = read_json_object_or_empty(path)?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| PluginError::Serialization("Claude MCP root must be an object".into()))?;
    object.insert("hasCompletedOnboarding".into(), Value::Bool(true));
    write_json_pretty(path, &root)
}

fn write_codex_provider(
    paths: &SwitchboardPaths,
    args: &ApplyProviderArgs,
) -> Result<(), PluginError> {
    let provider_id = args
        .provider_id
        .as_deref()
        .map(sanitize_provider_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CODEX_PROVIDER_ID.to_string());
    let model = defaulted(args.model.as_deref(), "gpt-5.4");
    let reasoning = defaulted(args.reasoning_effort.as_deref(), "high");
    let base_url = normalize_codex_base_url(&args.base_url);
    let mut auth = read_json_object_or_empty(&paths.codex_auth_path)?;
    let auth_object = auth.as_object_mut().ok_or_else(|| {
        PluginError::Serialization("Codex auth root must be an object".into())
    })?;
    auth_object.insert("OPENAI_API_KEY".into(), json!(args.api_key.trim()));
    let config_text = build_codex_config_text(
        &paths.codex_config_path,
        &provider_id,
        args.name.trim(),
        &base_url,
        &model,
        &reasoning,
    )?;

    write_codex_live_atomic(
        &paths.codex_auth_path,
        &paths.codex_config_path,
        &auth,
        &config_text,
    )
}

fn build_codex_config_text(
    config_path: &Path,
    provider_id: &str,
    name: &str,
    base_url: &str,
    model: &str,
    reasoning_effort: &str,
) -> Result<String, PluginError> {
    let mut doc = read_existing_codex_doc(config_path)?;
    apply_codex_provider_doc(
        &mut doc,
        provider_id,
        name,
        base_url,
        model,
        reasoning_effort,
    );

    let text = doc.to_string();
    text.parse::<DocumentMut>().map_err(|error| {
        PluginError::Serialization(format!("generated Codex config.toml is invalid: {error}"))
    })?;
    Ok(text)
}

fn read_existing_codex_doc(config_path: &Path) -> Result<DocumentMut, PluginError> {
    if !config_path.exists() {
        return Ok(DocumentMut::new());
    }

    let text = fs::read_to_string(config_path)?;
    if text.trim().is_empty() {
        return Ok(DocumentMut::new());
    }

    text.parse::<DocumentMut>().map_err(|error| {
        PluginError::Serialization(format!(
            "failed to parse existing Codex config.toml at {}: {error}",
            config_path.display()
        ))
    })
}

fn apply_codex_provider_doc(
    doc: &mut DocumentMut,
    provider_id: &str,
    name: &str,
    base_url: &str,
    model: &str,
    reasoning_effort: &str,
) {
    doc["model_provider"] = toml_edit::value(provider_id);
    doc["model"] = toml_edit::value(model);
    doc["model_reasoning_effort"] = toml_edit::value(reasoning_effort);
    doc["disable_response_storage"] = toml_edit::value(true);

    if !doc.as_table().contains_key("model_providers") {
        doc["model_providers"] = Item::Table(Table::new());
    }

    if provider_id != LEGACY_CODEX_PROVIDER_ID {
        if let Some(providers) = doc
            .get_mut("model_providers")
            .and_then(|item| item.as_table_like_mut())
        {
            providers.remove(LEGACY_CODEX_PROVIDER_ID);
        }
    }

    let mut provider = Table::new();
    provider["name"] = toml_edit::value(if name.is_empty() { provider_id } else { name });
    provider["base_url"] = toml_edit::value(base_url);
    provider["wire_api"] = toml_edit::value("responses");
    provider["requires_openai_auth"] = toml_edit::value(true);
    doc["model_providers"][provider_id] = Item::Table(provider);
}

fn write_codex_live_atomic(
    auth_path: &Path,
    config_path: &Path,
    auth: &Value,
    config_text: &str,
) -> Result<(), PluginError> {
    config_text.parse::<DocumentMut>().map_err(|error| {
        PluginError::Serialization(format!("Codex config.toml is invalid: {error}"))
    })?;

    let old_auth = read_optional_bytes(auth_path)?;
    let old_config = read_optional_bytes(config_path)?;
    if let Err(error) = write_json_pretty(auth_path, auth) {
        return Err(error);
    }

    if let Err(error) = atomic_write(config_path, config_text.as_bytes()) {
        restore_optional_bytes(auth_path, old_auth)?;
        restore_optional_bytes(config_path, old_config)?;
        return Err(error);
    }

    Ok(())
}

fn defaulted(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

pub fn sanitize_provider_id(value: &str) -> String {
    let mut out = String::new();
    let mut last_was_underscore = false;
    for ch in value.trim().to_ascii_lowercase().chars() {
        let next = if ch.is_ascii_alphanumeric() { ch } else { '_' };
        if next == '_' && last_was_underscore {
            continue;
        }
        last_was_underscore = next == '_';
        out.push(next);
    }
    out.trim_matches('_').to_string()
}

pub fn normalize_codex_base_url(value: &str) -> String {
    let trimmed = value.trim().trim_end_matches('/');
    if trimmed.ends_with("/v1") {
        return trimmed.to_string();
    }

    let without_scheme = trimmed
        .split_once("://")
        .map(|(_, rest)| rest)
        .unwrap_or(trimmed);
    if !without_scheme.contains('/') {
        format!("{trimmed}/v1")
    } else {
        trimmed.to_string()
    }
}

fn build_models_url(base_url: &str, models_path: Option<&str>) -> String {
    let base = normalize_codex_base_url(base_url);
    let path = models_path
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("/models");
    if path.starts_with("http://") || path.starts_with("https://") {
        return path.to_string();
    }
    let normalized_path = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{path}")
    };
    format!("{}{}", base.trim_end_matches('/'), normalized_path)
}

fn parse_model_ids(root: &Value) -> Vec<String> {
    let Some(data) = root.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut models = data
        .iter()
        .filter_map(|item| {
            item.get("id")
                .and_then(Value::as_str)
                .or_else(|| item.get("name").and_then(Value::as_str))
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string)
        })
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    models
}

fn http_get_json(url: &str, api_key: &str) -> Result<String, PluginError> {
    let response = ureq::get(url)
        .set("Authorization", &format!("Bearer {api_key}"))
        .set("Accept", "application/json")
        .timeout(std::time::Duration::from_secs(12))
        .call()
        .map_err(|error| PluginError::Custom(format!("models request failed: {error}")))?;
    response
        .into_string()
        .map_err(|error| PluginError::Io(format!("failed to read models response: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codex_base_url_adds_v1_only_to_origins() {
        assert_eq!(
            normalize_codex_base_url("https://api.example.com"),
            "https://api.example.com/v1"
        );
        assert_eq!(
            normalize_codex_base_url("https://api.example.com/custom"),
            "https://api.example.com/custom"
        );
        assert_eq!(
            normalize_codex_base_url("https://api.example.com/v1"),
            "https://api.example.com/v1"
        );
    }

    #[test]
    fn codex_provider_update_preserves_mcp_and_profiles() {
        let mut doc = r#"model_provider = "old"
model = "gpt-4"

[model_providers.old]
name = "Old"
base_url = "https://old.example/v1"
wire_api = "responses"

[mcp_servers.context7]
type = "stdio"
command = "npx"

[profiles.work]
model_provider = "old"
model = "gpt-4"
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        apply_codex_provider_doc(
            &mut doc,
            "switchboard",
            "Switchboard",
            "https://new.example/v1",
            "gpt-5.4",
            "high",
        );
        let parsed: toml_edit::DocumentMut = doc.to_string().parse().expect("valid output");

        assert!(parsed.get("mcp_servers").is_some());
        assert!(parsed.get("profiles").is_some());
        assert_eq!(
            parsed.get("model_provider").and_then(|item| item.as_str()),
            Some("switchboard")
        );
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|item| item.as_table())
                .and_then(|table| table.get("switchboard"))
                .and_then(|item| item.as_table())
                .and_then(|table| table.get("base_url"))
                .and_then(|item| item.as_str()),
            Some("https://new.example/v1")
        );
    }

    #[test]
    fn codex_provider_removes_legacy_switchboard_provider_when_using_new_id() {
        let mut doc = r#"model_provider = "switchboard"
model = "gpt-4"

[model_providers.switchboard]
name = "Switchboard"
base_url = "https://old.example/v1"
wire_api = "responses"

[model_providers.other]
name = "Other"
base_url = "https://other.example/v1"
wire_api = "responses"
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        apply_codex_provider_doc(
            &mut doc,
            "haloforge_gateway",
            "HaloForge Gateway",
            "https://new.example/v1",
            "gpt-5.4",
            "high",
        );

        let parsed: DocumentMut = doc.to_string().parse().expect("valid output");
        let providers = parsed
            .get("model_providers")
            .and_then(|item| item.as_table())
            .expect("providers table");
        assert!(providers.get("switchboard").is_none());
        assert!(providers.get("haloforge_gateway").is_some());
        assert!(providers.get("other").is_some());
    }
}
