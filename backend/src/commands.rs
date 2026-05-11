use hf_plugin_api::{PluginContext, PluginError};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table};

const CLAUDE_TARGET: &str = "claude";
const CODEX_TARGET: &str = "codex";
const BOTH_TARGET: &str = "both";
const STABLE_CODEX_PROVIDER_ID: &str = "switchboard";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PathStatus {
    label: String,
    path: String,
    exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetStatus {
    id: String,
    label: String,
    configured: bool,
    summary: Option<String>,
    paths: Vec<PathStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SwitchboardStatus {
    os: String,
    home_dir: Option<String>,
    data_dir: String,
    targets: Vec<TargetStatus>,
    backups: Vec<BackupInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    id: String,
    created_at: String,
    path: String,
    files: Vec<BackupFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    original_path: String,
    backup_file: Option<String>,
    existed: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProviderArgs {
    target: String,
    name: String,
    base_url: String,
    api_key: String,
    provider_id: Option<String>,
    model: Option<String>,
    reasoning_effort: Option<String>,
    haiku_model: Option<String>,
    sonnet_model: Option<String>,
    opus_model: Option<String>,
    set_claude_primary_api_key: Option<bool>,
    skip_claude_onboarding: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyProviderResult {
    backup: BackupInfo,
    changed_paths: Vec<String>,
    target: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupArgs {
    backup_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RestoreBackupResult {
    restored_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallMcpArgs {
    id: String,
    apps: Vec<String>,
    spec: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallMcpResult {
    changed_paths: Vec<String>,
}

pub fn switchboard_status(_args: Value, ctx: &dyn PluginContext) -> Result<Value, PluginError> {
    let paths = SwitchboardPaths::resolve()?;
    let status = SwitchboardStatus {
        os: std::env::consts::OS.to_string(),
        home_dir: paths.home.as_ref().map(|path| display_path(path)),
        data_dir: display_path(&ctx.data_dir()),
        targets: vec![claude_status(&paths), codex_status(&paths)],
        backups: list_backup_infos(ctx)?,
    };
    to_value(status)
}

pub fn switchboard_apply_provider(
    args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: ApplyProviderArgs = parse_args(args)?;
    validate_provider_args(&args)?;

    let paths = SwitchboardPaths::resolve()?;
    let mut touched = Vec::new();
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

    let backup = create_backup(ctx, &backup_paths)?;

    if applies_to(&args.target, CLAUDE_TARGET) {
        write_claude_provider(&paths, &args)?;
        touched.push(display_path(&paths.claude_settings_path));

        if args.set_claude_primary_api_key.unwrap_or(false) {
            set_claude_primary_api_key(&paths.claude_config_path)?;
            touched.push(display_path(&paths.claude_config_path));
        }

        if args.skip_claude_onboarding.unwrap_or(false) {
            set_claude_onboarding(&paths.claude_mcp_path)?;
            touched.push(display_path(&paths.claude_mcp_path));
        }
    }

    if applies_to(&args.target, CODEX_TARGET) {
        write_codex_provider(&paths, &args)?;
        touched.push(display_path(&paths.codex_auth_path));
        touched.push(display_path(&paths.codex_config_path));
    }

    let result = ApplyProviderResult {
        backup,
        changed_paths: touched,
        target: args.target,
    };
    to_value(result)
}

pub fn switchboard_list_backups(
    _args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    to_value(list_backup_infos(ctx)?)
}

pub fn switchboard_restore_backup(
    args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: RestoreBackupArgs = parse_args(args)?;
    let backup = read_backup_info(ctx, &args.backup_id)?;
    let backup_dir = backup_dir(ctx).join(&backup.id);
    let mut restored = Vec::new();

    for file in &backup.files {
        let original = PathBuf::from(&file.original_path);
        if file.existed {
            let rel = file
                .backup_file
                .as_deref()
                .ok_or_else(|| PluginError::Custom("backup manifest missing backup file".into()))?;
            let source = backup_dir.join(rel);
            if let Some(parent) = original.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&source, &original).map_err(|error| {
                PluginError::Io(format!(
                    "failed to restore {} from {}: {error}",
                    original.display(),
                    source.display()
                ))
            })?;
        } else if original.exists() {
            fs::remove_file(&original)?;
        }
        restored.push(display_path(&original));
    }

    to_value(RestoreBackupResult {
        restored_paths: restored,
    })
}

pub fn switchboard_install_mcp(args: Value, ctx: &dyn PluginContext) -> Result<Value, PluginError> {
    let args: InstallMcpArgs = parse_args(args)?;
    let id = sanitize_mcp_id(&args.id)?;
    validate_mcp_spec(&args.spec)?;

    let paths = SwitchboardPaths::resolve()?;
    let mut backup_paths = Vec::new();
    if args.apps.iter().any(|app| app == CLAUDE_TARGET) {
        backup_paths.push(paths.claude_mcp_path.clone());
    }
    if args.apps.iter().any(|app| app == CODEX_TARGET) {
        backup_paths.push(paths.codex_config_path.clone());
    }
    let _backup = create_backup(ctx, &backup_paths)?;

    let mut changed = Vec::new();
    if args.apps.iter().any(|app| app == CLAUDE_TARGET) {
        install_claude_mcp(&paths.claude_mcp_path, &id, args.spec.clone())?;
        changed.push(display_path(&paths.claude_mcp_path));
    }
    if args.apps.iter().any(|app| app == CODEX_TARGET) {
        install_codex_mcp(&paths.codex_config_path, &id, &args.spec)?;
        changed.push(display_path(&paths.codex_config_path));
    }

    to_value(InstallMcpResult {
        changed_paths: changed,
    })
}

#[derive(Debug, Clone)]
struct SwitchboardPaths {
    home: Option<PathBuf>,
    claude_settings_path: PathBuf,
    claude_config_path: PathBuf,
    claude_mcp_path: PathBuf,
    codex_auth_path: PathBuf,
    codex_config_path: PathBuf,
}

impl SwitchboardPaths {
    fn resolve() -> Result<Self, PluginError> {
        let home = dirs::home_dir().ok_or_else(|| {
            PluginError::Custom("unable to resolve the current user home directory".into())
        })?;

        let claude_dir = home.join(".claude");
        let settings = claude_dir.join("settings.json");
        let legacy_settings = claude_dir.join("claude.json");
        let claude_settings_path = if settings.exists() || !legacy_settings.exists() {
            settings
        } else {
            legacy_settings
        };

        let codex_dir = home.join(".codex");

        Ok(Self {
            home: Some(home.clone()),
            claude_settings_path,
            claude_config_path: claude_dir.join("config.json"),
            claude_mcp_path: home.join(".claude.json"),
            codex_auth_path: codex_dir.join("auth.json"),
            codex_config_path: codex_dir.join("config.toml"),
        })
    }
}

fn claude_status(paths: &SwitchboardPaths) -> TargetStatus {
    let summary = read_json(&paths.claude_settings_path)
        .ok()
        .and_then(|value| {
            let env = value.get("env")?.as_object()?;
            let base = env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str);
            let model = env.get("ANTHROPIC_MODEL").and_then(Value::as_str);
            let token = env
                .get("ANTHROPIC_AUTH_TOKEN")
                .or_else(|| env.get("ANTHROPIC_API_KEY"))
                .and_then(Value::as_str);
            Some(summary_parts([
                ("base", base.map(str::to_string)),
                ("model", model.map(str::to_string)),
                ("key", token.map(mask_secret)),
            ]))
        })
        .filter(|value| !value.is_empty());

    TargetStatus {
        id: CLAUDE_TARGET.to_string(),
        label: "Claude Code".to_string(),
        configured: paths.claude_settings_path.exists(),
        summary,
        paths: vec![
            path_status("settings", &paths.claude_settings_path),
            path_status("config", &paths.claude_config_path),
            path_status("mcp", &paths.claude_mcp_path),
        ],
    }
}

fn codex_status(paths: &SwitchboardPaths) -> TargetStatus {
    let mut parts = Vec::new();
    if let Ok(auth) = read_json(&paths.codex_auth_path) {
        if let Some(key) = auth.get("OPENAI_API_KEY").and_then(Value::as_str) {
            parts.push(format!("key={}", mask_secret(key)));
        }
    }
    if let Ok(config) = fs::read_to_string(&paths.codex_config_path) {
        if let Ok(doc) = config.parse::<DocumentMut>() {
            if let Some(provider) = doc.get("model_provider").and_then(|item| item.as_str()) {
                parts.push(format!("provider={provider}"));
                if let Some(base_url) = doc
                    .get("model_providers")
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get(provider))
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get("base_url"))
                    .and_then(|item| item.as_str())
                {
                    parts.push(format!("base={base_url}"));
                }
            }
            if let Some(model) = doc.get("model").and_then(|item| item.as_str()) {
                parts.push(format!("model={model}"));
            }
        }
    }

    TargetStatus {
        id: CODEX_TARGET.to_string(),
        label: "Codex".to_string(),
        configured: paths.codex_auth_path.exists() || paths.codex_config_path.exists(),
        summary: (!parts.is_empty()).then(|| parts.join(" | ")),
        paths: vec![
            path_status("auth", &paths.codex_auth_path),
            path_status("config", &paths.codex_config_path),
        ],
    }
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
        .unwrap_or_else(|| STABLE_CODEX_PROVIDER_ID.to_string());
    let model = defaulted(args.model.as_deref(), "gpt-5.4");
    let reasoning = defaulted(args.reasoning_effort.as_deref(), "high");
    let codex_base_url = normalize_codex_base_url(&args.base_url);
    let auth = json!({ "OPENAI_API_KEY": args.api_key.trim() });
    let config_text = build_codex_config(
        &provider_id,
        args.name.trim(),
        &codex_base_url,
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

fn build_codex_config(
    provider_id: &str,
    name: &str,
    base_url: &str,
    model: &str,
    reasoning_effort: &str,
) -> Result<String, PluginError> {
    let mut doc = DocumentMut::new();
    doc["model_provider"] = toml_edit::value(provider_id);
    doc["model"] = toml_edit::value(model);
    doc["model_reasoning_effort"] = toml_edit::value(reasoning_effort);
    doc["disable_response_storage"] = toml_edit::value(true);

    let mut provider = Table::new();
    provider["name"] = toml_edit::value(if name.is_empty() { provider_id } else { name });
    provider["base_url"] = toml_edit::value(base_url);
    provider["wire_api"] = toml_edit::value("responses");
    provider["requires_openai_auth"] = toml_edit::value(true);

    let mut providers = Table::new();
    providers[provider_id] = Item::Table(provider);
    doc["model_providers"] = Item::Table(providers);

    let text = doc.to_string();
    text.parse::<DocumentMut>().map_err(|error| {
        PluginError::Serialization(format!("generated Codex config.toml is invalid: {error}"))
    })?;
    Ok(text)
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
    let auth_result = write_json_pretty(auth_path, auth);
    if let Err(error) = auth_result {
        return Err(error);
    }

    if let Err(error) = atomic_write(config_path, config_text.as_bytes()) {
        restore_optional_bytes(auth_path, old_auth)?;
        restore_optional_bytes(config_path, old_config)?;
        return Err(error);
    }

    Ok(())
}

fn install_claude_mcp(path: &Path, id: &str, mut spec: Value) -> Result<(), PluginError> {
    wrap_stdio_command_for_windows(&mut spec);
    let mut root = read_json_object_or_empty(path)?;
    let object = root
        .as_object_mut()
        .ok_or_else(|| PluginError::Serialization("Claude MCP root must be an object".into()))?;

    let servers = object
        .entry("mcpServers")
        .or_insert_with(|| Value::Object(Map::new()));
    if !servers.is_object() {
        *servers = Value::Object(Map::new());
    }
    let server_map = servers
        .as_object_mut()
        .ok_or_else(|| PluginError::Serialization("mcpServers must be an object".into()))?;
    server_map.insert(id.to_string(), spec);

    write_json_pretty(path, &root)
}

fn install_codex_mcp(path: &Path, id: &str, spec: &Value) -> Result<(), PluginError> {
    let mut doc = if path.exists() {
        let text = fs::read_to_string(path)?;
        if text.trim().is_empty() {
            DocumentMut::new()
        } else {
            text.parse::<DocumentMut>().map_err(|error| {
                PluginError::Serialization(format!("failed to parse Codex config.toml: {error}"))
            })?
        }
    } else {
        DocumentMut::new()
    };

    if let Some(mcp) = doc.get_mut("mcp").and_then(|item| item.as_table_like_mut()) {
        mcp.remove("servers");
    }

    if !doc.as_table().contains_key("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }
    doc["mcp_servers"][id] = Item::Table(json_mcp_to_toml(spec)?);

    atomic_write(path, doc.to_string().as_bytes())
}

fn json_mcp_to_toml(spec: &Value) -> Result<Table, PluginError> {
    let object = spec
        .as_object()
        .ok_or_else(|| PluginError::Serialization("MCP spec must be an object".into()))?;
    let typ = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("stdio");

    let mut table = Table::new();
    table["type"] = toml_edit::value(typ);
    match typ {
        "stdio" => {
            let command = object
                .get("command")
                .and_then(Value::as_str)
                .ok_or_else(|| PluginError::Custom("stdio MCP spec requires command".into()))?;
            table["command"] = toml_edit::value(command);

            if let Some(args) = object.get("args").and_then(Value::as_array) {
                let mut array = Array::new();
                for arg in args.iter().filter_map(Value::as_str) {
                    array.push(arg);
                }
                if !array.is_empty() {
                    table["args"] = Item::Value(toml_edit::Value::Array(array));
                }
            }
            if let Some(cwd) = object.get("cwd").and_then(Value::as_str) {
                if !cwd.trim().is_empty() {
                    table["cwd"] = toml_edit::value(cwd);
                }
            }
            if let Some(env) = object.get("env").and_then(Value::as_object) {
                let mut env_table = Table::new();
                for (key, value) in env {
                    if let Some(value) = value.as_str() {
                        env_table[key.as_str()] = toml_edit::value(value);
                    }
                }
                if !env_table.is_empty() {
                    table["env"] = Item::Table(env_table);
                }
            }
        }
        "http" | "sse" => {
            let url = object
                .get("url")
                .and_then(Value::as_str)
                .ok_or_else(|| PluginError::Custom("http/sse MCP spec requires url".into()))?;
            table["url"] = toml_edit::value(url);
            if let Some(headers) = object.get("headers").and_then(Value::as_object) {
                let mut header_table = Table::new();
                for (key, value) in headers {
                    if let Some(value) = value.as_str() {
                        header_table[key.as_str()] = toml_edit::value(value);
                    }
                }
                if !header_table.is_empty() {
                    table["http_headers"] = Item::Table(header_table);
                }
            }
        }
        _ => return Err(PluginError::Custom(format!("unsupported MCP type: {typ}"))),
    }

    let handled = match typ {
        "stdio" => ["type", "command", "args", "cwd", "env"].as_slice(),
        "http" | "sse" => ["type", "url", "headers"].as_slice(),
        _ => ["type"].as_slice(),
    };
    for (key, value) in object {
        if handled.contains(&key.as_str()) {
            continue;
        }
        if let Some(item) = json_value_to_toml_item(value) {
            table[key.as_str()] = item;
        }
    }

    Ok(table)
}

fn json_value_to_toml_item(value: &Value) -> Option<Item> {
    match value {
        Value::String(value) => Some(toml_edit::value(value.as_str())),
        Value::Number(value) => value
            .as_i64()
            .map(toml_edit::value)
            .or_else(|| value.as_f64().map(toml_edit::value)),
        Value::Bool(value) => Some(toml_edit::value(*value)),
        Value::Array(values) => {
            let mut array = Array::new();
            for value in values {
                match value {
                    Value::String(value) => array.push(value.as_str()),
                    Value::Number(value) if value.is_i64() => array.push(value.as_i64()?),
                    Value::Number(value) if value.is_f64() => array.push(value.as_f64()?),
                    Value::Bool(value) => array.push(*value),
                    _ => return None,
                }
            }
            (!array.is_empty()).then_some(Item::Value(toml_edit::Value::Array(array)))
        }
        Value::Object(object) => {
            let mut table = InlineTable::new();
            for (key, value) in object {
                table.insert(key, value.as_str()?.into());
            }
            (!table.is_empty()).then_some(Item::Value(toml_edit::Value::InlineTable(table)))
        }
        Value::Null => None,
    }
}

fn validate_provider_args(args: &ApplyProviderArgs) -> Result<(), PluginError> {
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

fn validate_mcp_spec(spec: &Value) -> Result<(), PluginError> {
    let typ = spec.get("type").and_then(Value::as_str).unwrap_or("stdio");
    match typ {
        "stdio" => {
            let command = spec.get("command").and_then(Value::as_str).unwrap_or("");
            if command.trim().is_empty() {
                return Err(PluginError::Custom(
                    "stdio MCP spec requires command".into(),
                ));
            }
        }
        "http" | "sse" => {
            let url = spec.get("url").and_then(Value::as_str).unwrap_or("");
            if url.trim().is_empty() {
                return Err(PluginError::Custom("http/sse MCP spec requires url".into()));
            }
        }
        _ => return Err(PluginError::Custom(format!("unsupported MCP type: {typ}"))),
    }
    Ok(())
}

fn create_backup(ctx: &dyn PluginContext, paths: &[PathBuf]) -> Result<BackupInfo, PluginError> {
    let id = timestamp_id();
    let dir = backup_dir(ctx).join(&id);
    let files_dir = dir.join("files");
    fs::create_dir_all(&files_dir)?;

    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    for path in paths {
        let path_text = display_path(path);
        if !seen.insert(path_text.clone()) {
            continue;
        }
        let index = files.len();
        if path.exists() {
            let backup_file = format!("files/{index}");
            fs::copy(path, dir.join(&backup_file)).map_err(|error| {
                PluginError::Io(format!("failed to back up {}: {error}", path.display()))
            })?;
            files.push(BackupFile {
                original_path: path_text,
                backup_file: Some(backup_file),
                existed: true,
            });
        } else {
            files.push(BackupFile {
                original_path: path_text,
                backup_file: None,
                existed: false,
            });
        }
    }

    let backup = BackupInfo {
        id: id.clone(),
        created_at: id,
        path: display_path(&dir),
        files,
    };
    write_json_pretty(&dir.join("manifest.json"), &backup)?;
    Ok(backup)
}

fn list_backup_infos(ctx: &dyn PluginContext) -> Result<Vec<BackupInfo>, PluginError> {
    let dir = backup_dir(ctx);
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut backups = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let manifest_path = entry.path().join("manifest.json");
        if manifest_path.exists() {
            if let Ok(info) = read_json_typed::<BackupInfo>(&manifest_path) {
                backups.push(info);
            }
        }
    }
    backups.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(backups)
}

fn read_backup_info(ctx: &dyn PluginContext, backup_id: &str) -> Result<BackupInfo, PluginError> {
    let safe = sanitize_backup_id(backup_id)?;
    read_json_typed(&backup_dir(ctx).join(safe).join("manifest.json"))
}

fn backup_dir(ctx: &dyn PluginContext) -> PathBuf {
    ctx.data_dir().join("backups")
}

fn sanitize_backup_id(value: &str) -> Result<String, PluginError> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.contains("..")
    {
        return Err(PluginError::Custom("invalid backup id".into()));
    }
    Ok(trimmed.to_string())
}

fn sanitize_mcp_id(id: &str) -> Result<String, PluginError> {
    let id = sanitize_provider_id(id);
    if id.is_empty() {
        return Err(PluginError::Custom("MCP id is required".into()));
    }
    Ok(id)
}

fn sanitize_provider_id(value: &str) -> String {
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

fn normalize_codex_base_url(value: &str) -> String {
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

fn applies_to(target: &str, app: &str) -> bool {
    target == app || target == BOTH_TARGET
}

fn read_json(path: &Path) -> Result<Value, PluginError> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        PluginError::Serialization(format!("failed to parse {}: {error}", path.display()))
    })
}

fn read_json_object_or_empty(path: &Path) -> Result<Value, PluginError> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    match read_json(path)? {
        Value::Object(map) => Ok(Value::Object(map)),
        _ => Err(PluginError::Serialization(format!(
            "{} root must be a JSON object",
            path.display()
        ))),
    }
}

fn read_json_typed<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, PluginError> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        PluginError::Serialization(format!("failed to parse {}: {error}", path.display()))
    })
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), PluginError> {
    let content = serde_json::to_vec_pretty(value)
        .map_err(|error| PluginError::Serialization(error.to_string()))?;
    let mut content = content;
    content.push(b'\n');
    atomic_write(path, &content)
}

fn atomic_write(path: &Path, content: &[u8]) -> Result<(), PluginError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let parent = path
        .parent()
        .ok_or_else(|| PluginError::Custom("invalid path".into()))?;
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| PluginError::Custom("invalid file name".into()))?;
    let tmp = parent.join(format!("{filename}.tmp.{}", timestamp_id()));

    {
        let mut file = fs::File::create(&tmp)?;
        file.write_all(content)?;
        file.flush()?;
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = fs::metadata(path) {
            let _ =
                fs::set_permissions(&tmp, fs::Permissions::from_mode(meta.permissions().mode()));
        }
    }

    #[cfg(windows)]
    {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }

    fs::rename(&tmp, path).map_err(|error| {
        PluginError::Io(format!(
            "failed to replace {} with {}: {error}",
            path.display(),
            tmp.display()
        ))
    })
}

fn read_optional_bytes(path: &Path) -> Result<Option<Vec<u8>>, PluginError> {
    if path.exists() {
        Ok(Some(fs::read(path)?))
    } else {
        Ok(None)
    }
}

fn restore_optional_bytes(path: &Path, bytes: Option<Vec<u8>>) -> Result<(), PluginError> {
    match bytes {
        Some(bytes) => atomic_write(path, &bytes),
        None => {
            if path.exists() {
                fs::remove_file(path)?;
            }
            Ok(())
        }
    }
}

fn wrap_stdio_command_for_windows(spec: &mut Value) {
    if !cfg!(windows) {
        return;
    }
    let Some(object) = spec.as_object_mut() else {
        return;
    };
    let typ = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("stdio");
    if typ != "stdio" {
        return;
    }
    let Some(command) = object.get("command").and_then(Value::as_str) else {
        return;
    };
    if command.eq_ignore_ascii_case("cmd") || command.eq_ignore_ascii_case("cmd.exe") {
        return;
    }
    let stem = Path::new(command)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(command)
        .to_ascii_lowercase();
    let needs_wrap = ["npx", "npm", "yarn", "pnpm", "node", "bun", "deno"]
        .iter()
        .any(|item| *item == stem);
    if !needs_wrap {
        return;
    }

    let original_args = object
        .get("args")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut next_args = vec![json!("/c"), json!(command)];
    next_args.extend(original_args);
    object.insert("command".into(), json!("cmd"));
    object.insert("args".into(), Value::Array(next_args));
}

fn path_status(label: &str, path: &Path) -> PathStatus {
    PathStatus {
        label: label.to_string(),
        path: display_path(path),
        exists: path.exists(),
    }
}

fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn mask_secret(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.len() <= 8 {
        return "****".to_string();
    }
    format!("{}...{}", &trimmed[..4], &trimmed[trimmed.len() - 4..])
}

fn summary_parts(parts: impl IntoIterator<Item = (&'static str, Option<String>)>) -> String {
    parts
        .into_iter()
        .filter_map(|(key, value)| value.map(|value| format!("{key}={value}")))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn defaulted(value: Option<&str>, fallback: &str) -> String {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

fn timestamp_id() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    millis.to_string()
}

fn parse_args<T: for<'de> Deserialize<'de>>(args: Value) -> Result<T, PluginError> {
    serde_json::from_value(args).map_err(|error| PluginError::Serialization(error.to_string()))
}

fn to_value<T: Serialize>(value: T) -> Result<Value, PluginError> {
    serde_json::to_value(value).map_err(|error| PluginError::Serialization(error.to_string()))
}
