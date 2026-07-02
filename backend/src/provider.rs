use crate::fs_util::{
    atomic_write, read_json_object_or_empty, read_optional_bytes, restore_optional_bytes,
    write_json_pretty,
};
use crate::paths::SwitchboardPaths;
use crate::types::{
    ApplyProviderArgs, CleanupCodexArgs, DiscoverModelsArgs, DiscoverModelsResult, BOTH_TARGET,
    CLAUDE_TARGET, CODEX_TARGET, DEFAULT_CODEX_PROVIDER_ID, LEGACY_CODEX_PROVIDER_ID,
    PREVIOUS_DEFAULT_CODEX_PROVIDER_ID,
};
use hf_plugin_api::PluginError;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::Path;
use toml_edit::{DocumentMut, Item, Table};

const CODEX_BUILTIN_PLUGINS: &[(&str, &str)] = &[
    ("browser@openai-bundled", "Browser"),
    ("chrome@openai-bundled", "Chrome"),
    ("computer-use@openai-bundled", "Computer Use"),
    ("documents@openai-primary-runtime", "Documents"),
    ("spreadsheets@openai-primary-runtime", "Spreadsheets"),
    ("presentations@openai-primary-runtime", "Presentations"),
    ("github@openai-curated", "GitHub"),
];

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

pub fn codex_cleanup_backup_paths(paths: &SwitchboardPaths) -> Vec<std::path::PathBuf> {
    vec![
        paths.codex_auth_path.clone(),
        paths.codex_config_path.clone(),
    ]
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

pub fn cleanup_codex_custom_api(
    paths: &SwitchboardPaths,
    args: &CleanupCodexArgs,
) -> Result<Vec<std::path::PathBuf>, PluginError> {
    let provider_id = args
        .provider_id
        .as_deref()
        .map(sanitize_provider_id)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_CODEX_PROVIDER_ID.to_string());

    let auth_changed = cleanup_codex_auth(&paths.codex_auth_path)?;
    let config_changed = cleanup_codex_config(&paths.codex_config_path, &provider_id)?;

    let mut touched = Vec::new();
    if auth_changed {
        touched.push(paths.codex_auth_path.clone());
    }
    if config_changed {
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

fn cleanup_codex_auth(path: &Path) -> Result<bool, PluginError> {
    if !path.exists() {
        return Ok(false);
    }

    let mut root = read_json_object_or_empty(path)?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| PluginError::Serialization("Codex auth root must be an object".into()))?;

    let had_api_key = object.remove("OPENAI_API_KEY").is_some();
    if had_api_key {
        object
            .entry("auth_mode")
            .or_insert_with(|| json!("chatgpt"));
        write_json_pretty(path, &root)?;
    }

    Ok(had_api_key)
}

fn cleanup_codex_config(path: &Path, provider_id: &str) -> Result<bool, PluginError> {
    if !path.exists() {
        return Ok(false);
    }

    let mut doc = read_existing_codex_doc(path)?;
    let changed = cleanup_codex_config_doc(&mut doc, provider_id);
    if changed {
        let text = doc.to_string();
        text.parse::<DocumentMut>().map_err(|error| {
            PluginError::Serialization(format!("generated Codex config.toml is invalid: {error}"))
        })?;
        atomic_write(path, text.as_bytes())?;
    }
    Ok(changed)
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
    let auth_object = auth
        .as_object_mut()
        .ok_or_else(|| PluginError::Serialization("Codex auth root must be an object".into()))?;
    if args.preserve_codex_chatgpt_auth.unwrap_or(false) {
        auth_object.insert("auth_mode".into(), json!("chatgpt"));
        auth_object.insert("OPENAI_API_KEY".into(), Value::Null);
    } else {
        auth_object.insert("OPENAI_API_KEY".into(), json!(args.api_key.trim()));
    }
    let config_text = build_codex_config_text(
        &paths.codex_config_path,
        &provider_id,
        args.name.trim(),
        &base_url,
        &model,
        &reasoning,
        args.api_key.trim(),
        args.enable_codex_builtin_plugins.unwrap_or(false),
        args.preserve_codex_chatgpt_auth.unwrap_or(false),
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
    api_key: &str,
    enable_builtin_plugins: bool,
    preserve_chatgpt_auth: bool,
) -> Result<String, PluginError> {
    let mut doc = read_existing_codex_doc(config_path)?;
    apply_codex_provider_doc(
        &mut doc,
        provider_id,
        name,
        base_url,
        model,
        reasoning_effort,
        api_key,
        enable_builtin_plugins,
        preserve_chatgpt_auth,
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
    api_key: &str,
    enable_builtin_plugins: bool,
    preserve_chatgpt_auth: bool,
) {
    doc["model_provider"] = toml_edit::value(provider_id);
    doc["model"] = toml_edit::value(model);
    doc["model_reasoning_effort"] = toml_edit::value(reasoning_effort);
    doc["disable_response_storage"] = toml_edit::value(true);

    if provider_id == DEFAULT_CODEX_PROVIDER_ID {
        doc["openai_base_url"] = toml_edit::value(base_url);
        if preserve_chatgpt_auth {
            doc["experimental_bearer_token"] = toml_edit::value(api_key);
        } else {
            doc.as_table_mut().remove("experimental_bearer_token");
        }
        remove_managed_codex_provider_tables(doc);
        apply_codex_builtin_plugins(doc, enable_builtin_plugins);
        return;
    }

    doc.as_table_mut().remove("openai_base_url");
    doc.as_table_mut().remove("experimental_bearer_token");

    if !doc.as_table().contains_key("model_providers") {
        doc["model_providers"] = Item::Table(Table::new());
    }

    remove_legacy_codex_provider_tables(doc, provider_id);

    let mut provider = Table::new();
    provider["name"] = toml_edit::value(if name.is_empty() { provider_id } else { name });
    provider["base_url"] = toml_edit::value(base_url);
    provider["wire_api"] = toml_edit::value("responses");
    provider["requires_openai_auth"] = toml_edit::value(true);
    if preserve_chatgpt_auth {
        provider["experimental_bearer_token"] = toml_edit::value(api_key);
    }
    doc["model_providers"][provider_id] = Item::Table(provider);

    apply_codex_builtin_plugins(doc, enable_builtin_plugins);
}

fn apply_codex_builtin_plugins(doc: &mut DocumentMut, enable_builtin_plugins: bool) {
    if !enable_builtin_plugins {
        return;
    }
    if !doc.as_table().contains_key("plugins") {
        doc["plugins"] = Item::Table(Table::new());
    }
    if let Some(plugins) = doc
        .get_mut("plugins")
        .and_then(|item| item.as_table_like_mut())
    {
        for (plugin_id, _) in CODEX_BUILTIN_PLUGINS {
            enable_codex_plugin(plugins, plugin_id);
        }
    }
}

fn remove_legacy_codex_provider_tables(doc: &mut DocumentMut, active_provider_id: &str) {
    if let Some(providers) = doc
        .get_mut("model_providers")
        .and_then(|item| item.as_table_like_mut())
    {
        for legacy in [LEGACY_CODEX_PROVIDER_ID, PREVIOUS_DEFAULT_CODEX_PROVIDER_ID] {
            if active_provider_id != legacy {
                providers.remove(legacy);
            }
        }
    }
}

fn remove_managed_codex_provider_tables(doc: &mut DocumentMut) {
    if let Some(providers) = doc
        .get_mut("model_providers")
        .and_then(|item| item.as_table_like_mut())
    {
        for managed in [
            DEFAULT_CODEX_PROVIDER_ID,
            PREVIOUS_DEFAULT_CODEX_PROVIDER_ID,
            LEGACY_CODEX_PROVIDER_ID,
        ] {
            providers.remove(managed);
        }
    }
    let providers_empty = doc
        .get("model_providers")
        .and_then(|item| item.as_table())
        .map(|table| table.is_empty())
        .unwrap_or(false);
    if providers_empty {
        doc.as_table_mut().remove("model_providers");
    }
}

fn enable_codex_plugin(plugins: &mut dyn toml_edit::TableLike, plugin_id: &str) {
    if let Some(plugin) = plugins
        .get_mut(plugin_id)
        .and_then(|item| item.as_table_like_mut())
    {
        plugin.insert("enabled", toml_edit::value(true));
    } else {
        let mut plugin = Table::new();
        plugin["enabled"] = toml_edit::value(true);
        plugins.insert(plugin_id, Item::Table(plugin));
    }
}

fn cleanup_codex_config_doc(doc: &mut DocumentMut, provider_id: &str) -> bool {
    let mut changed = false;
    let provider_ids = cleanup_provider_ids(provider_id);
    let active_is_managed = doc
        .get("model_provider")
        .and_then(|item| item.as_str())
        .map(|current| provider_ids.iter().any(|id| id == current))
        .unwrap_or(false);

    if let Some(current) = doc.get("model_provider").and_then(|item| item.as_str()) {
        if provider_ids.iter().any(|id| id == current) {
            doc.as_table_mut().remove("model_provider");
            changed = true;
        }
    }

    if active_is_managed {
        if doc.as_table_mut().remove("model").is_some() {
            changed = true;
        }
        if doc
            .as_table_mut()
            .remove("model_reasoning_effort")
            .is_some()
        {
            changed = true;
        }
        if doc
            .as_table_mut()
            .remove("disable_response_storage")
            .is_some()
        {
            changed = true;
        }
        if doc.as_table_mut().remove("openai_base_url").is_some() {
            changed = true;
        }
        if doc
            .as_table_mut()
            .remove("experimental_bearer_token")
            .is_some()
        {
            changed = true;
        }
    }

    if let Some(providers) = doc
        .get_mut("model_providers")
        .and_then(|item| item.as_table_like_mut())
    {
        for id in &provider_ids {
            if providers.remove(id).is_some() {
                changed = true;
            }
        }
    }

    if let Some(plugins) = doc
        .get_mut("plugins")
        .and_then(|item| item.as_table_like_mut())
    {
        for (plugin_id, _) in CODEX_BUILTIN_PLUGINS {
            if plugins.remove(plugin_id).is_some() {
                changed = true;
            }
        }
    }

    changed
}

fn cleanup_provider_ids(provider_id: &str) -> Vec<String> {
    let mut ids = vec![provider_id.to_string()];
    for legacy in [
        DEFAULT_CODEX_PROVIDER_ID,
        PREVIOUS_DEFAULT_CODEX_PROVIDER_ID,
        LEGACY_CODEX_PROVIDER_ID,
    ] {
        if !ids.iter().any(|id| id == legacy) {
            ids.push(legacy.to_string());
        }
    }
    ids
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
            "sk-test",
            false,
            false,
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
            "sk-test",
            false,
            false,
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

    #[test]
    fn codex_provider_can_enable_chrome_plugin_and_bearer_token() {
        let mut doc = r#"model_provider = "old"

[plugins."browser@openai-bundled"]
enabled = true
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        apply_codex_provider_doc(
            &mut doc,
            "third_party",
            "Third Party",
            "https://new.example/v1",
            "gpt-5.5",
            "high",
            "sk-third-party",
            true,
            true,
        );

        let parsed: DocumentMut = doc.to_string().parse().expect("valid output");
        assert_eq!(
            parsed
                .get("model_providers")
                .and_then(|item| item.as_table())
                .and_then(|table| table.get("third_party"))
                .and_then(|item| item.as_table())
                .and_then(|table| table.get("experimental_bearer_token"))
                .and_then(|item| item.as_str()),
            Some("sk-third-party")
        );
        assert_eq!(
            parsed
                .get("plugins")
                .and_then(|item| item.as_table())
                .and_then(|table| table.get("chrome@openai-bundled"))
                .and_then(|item| item.as_table())
                .and_then(|table| table.get("enabled"))
                .and_then(|item| item.as_bool()),
            Some(true)
        );
        assert!(parsed
            .get("plugins")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("browser@openai-bundled"))
            .is_some());
    }

    #[test]
    fn codex_provider_uses_openai_base_url_for_official_default() {
        let mut doc = r#"model_provider = "haloforge_gateway"

[model_providers.haloforge_gateway]
name = "Old"
base_url = "https://old.example/v1"
wire_api = "responses"
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        apply_codex_provider_doc(
            &mut doc,
            DEFAULT_CODEX_PROVIDER_ID,
            "OpenAI Official",
            "https://api.example/v1",
            "gpt-5.5",
            "high",
            "sk-test",
            true,
            false,
        );

        let parsed: DocumentMut = doc.to_string().parse().expect("valid output");
        assert_eq!(
            parsed.get("model_provider").and_then(|item| item.as_str()),
            Some("openai")
        );
        assert_eq!(
            parsed.get("openai_base_url").and_then(|item| item.as_str()),
            Some("https://api.example/v1")
        );
        assert!(parsed
            .get("model_providers")
            .and_then(|item| item.as_table())
            .is_none());
        assert!(parsed
            .get("plugins")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("github@openai-curated"))
            .is_some());
    }

    #[test]
    fn codex_cleanup_removes_managed_openai_override() {
        let mut doc = r#"model_provider = "openai"
model = "gpt-5.5"
model_reasoning_effort = "high"
openai_base_url = "https://api.example/v1"
experimental_bearer_token = "sk-test"
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        assert!(cleanup_codex_config_doc(
            &mut doc,
            DEFAULT_CODEX_PROVIDER_ID
        ));
        let parsed: DocumentMut = doc.to_string().parse().expect("valid output");
        assert!(parsed.get("model_provider").is_none());
        assert!(parsed.get("model").is_none());
        assert!(parsed.get("openai_base_url").is_none());
        assert!(parsed.get("experimental_bearer_token").is_none());
    }

    #[test]
    fn codex_cleanup_removes_managed_custom_api_without_touching_mcp() {
        let mut doc = r#"model_provider = "haloforge_gateway"
model = "gpt-5.5"
model_reasoning_effort = "high"
disable_response_storage = true

[model_providers.haloforge_gateway]
name = "HaloForge Gateway"
base_url = "https://new.example/v1"
wire_api = "responses"
requires_openai_auth = true
experimental_bearer_token = "sk-test"

[plugins."chrome@openai-bundled"]
enabled = true

[mcp_servers.context7]
type = "stdio"
command = "npx"

[profiles.work]
model_provider = "other"
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        assert!(cleanup_codex_config_doc(&mut doc, "haloforge_gateway"));
        let parsed: DocumentMut = doc.to_string().parse().expect("valid output");
        assert!(parsed.get("model_provider").is_none());
        assert!(parsed.get("model").is_none());
        assert!(parsed
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("haloforge_gateway"))
            .is_none());
        assert!(parsed
            .get("plugins")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("chrome@openai-bundled"))
            .is_none());
        assert!(parsed.get("mcp_servers").is_some());
        assert!(parsed.get("profiles").is_some());
    }

    #[test]
    fn codex_cleanup_preserves_unmanaged_active_model_settings() {
        let mut doc = r#"model_provider = "other"
model = "gpt-4"

[model_providers.haloforge_gateway]
name = "HaloForge Gateway"
base_url = "https://new.example/v1"

[model_providers.other]
name = "Other"
base_url = "https://other.example/v1"
"#
        .parse::<DocumentMut>()
        .expect("valid toml");

        assert!(cleanup_codex_config_doc(&mut doc, "haloforge_gateway"));
        let parsed: DocumentMut = doc.to_string().parse().expect("valid output");
        assert_eq!(
            parsed.get("model_provider").and_then(|item| item.as_str()),
            Some("other")
        );
        assert_eq!(
            parsed.get("model").and_then(|item| item.as_str()),
            Some("gpt-4")
        );
        assert!(parsed
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("haloforge_gateway"))
            .is_none());
        assert!(parsed
            .get("model_providers")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("other"))
            .is_some());
    }
}
