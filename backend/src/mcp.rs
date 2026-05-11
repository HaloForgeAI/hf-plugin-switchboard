use crate::fs_util::{atomic_write, read_json_object_or_empty, write_json_pretty};
use crate::paths::SwitchboardPaths;
use crate::provider::{applies_to, sanitize_provider_id};
use crate::types::{InstallMcpArgs, CLAUDE_TARGET, CODEX_TARGET};
use hf_plugin_api::PluginError;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};
use toml_edit::{Array, DocumentMut, InlineTable, Item, Table};

pub fn validate_mcp_args(args: &InstallMcpArgs) -> Result<(), PluginError> {
    if sanitize_mcp_id(&args.id)?.is_empty() {
        return Err(PluginError::Custom("MCP id is required".into()));
    }
    if args.apps.is_empty() {
        return Err(PluginError::Custom(
            "select at least one target for MCP install".into(),
        ));
    }
    for app in &args.apps {
        if !matches!(app.as_str(), CLAUDE_TARGET | CODEX_TARGET | "both") {
            return Err(PluginError::Custom(format!(
                "unsupported MCP target '{}'",
                app
            )));
        }
    }
    validate_mcp_spec(&args.spec)
}

pub fn mcp_backup_paths(paths: &SwitchboardPaths, args: &InstallMcpArgs) -> Vec<PathBuf> {
    let mut backup_paths = Vec::new();
    if app_requested(&args.apps, CLAUDE_TARGET) {
        backup_paths.push(paths.claude_mcp_path.clone());
    }
    if app_requested(&args.apps, CODEX_TARGET) {
        backup_paths.push(paths.codex_config_path.clone());
    }
    backup_paths
}

pub fn install_mcp(
    paths: &SwitchboardPaths,
    args: &InstallMcpArgs,
) -> Result<Vec<PathBuf>, PluginError> {
    let id = sanitize_mcp_id(&args.id)?;
    let spec = normalize_mcp_spec(&args.spec)?;
    let mut touched = Vec::new();

    if app_requested(&args.apps, CLAUDE_TARGET) {
        install_claude_mcp(&paths.claude_mcp_path, &id, &spec)?;
        touched.push(paths.claude_mcp_path.clone());
    }
    if app_requested(&args.apps, CODEX_TARGET) {
        install_codex_mcp(&paths.codex_config_path, &id, &spec)?;
        touched.push(paths.codex_config_path.clone());
    }

    Ok(touched)
}

fn app_requested(apps: &[String], app: &str) -> bool {
    apps.iter().any(|value| applies_to(value, app))
}

fn install_claude_mcp(path: &Path, id: &str, spec: &Value) -> Result<(), PluginError> {
    let mut spec = spec.clone();
    normalize_stdio_command_for_windows(&mut spec);
    normalize_headers_for_claude(&mut spec);

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
    let mut spec = spec.clone();
    normalize_stdio_command_for_windows(&mut spec);
    normalize_headers_for_codex(&mut spec);

    let mut doc = read_codex_doc(path)?;

    if let Some(mcp) = doc.get_mut("mcp").and_then(|item| item.as_table_like_mut()) {
        mcp.remove("servers");
    }

    if !doc.as_table().contains_key("mcp_servers") {
        doc["mcp_servers"] = toml_edit::table();
    }
    doc["mcp_servers"][id] = Item::Table(json_mcp_to_toml(&spec)?);

    let text = doc.to_string();
    text.parse::<DocumentMut>().map_err(|error| {
        PluginError::Serialization(format!("generated Codex config.toml is invalid: {error}"))
    })?;
    atomic_write(path, text.as_bytes())
}

fn read_codex_doc(path: &Path) -> Result<DocumentMut, PluginError> {
    if !path.exists() {
        return Ok(DocumentMut::new());
    }

    let text = fs::read_to_string(path)?;
    if text.trim().is_empty() {
        return Ok(DocumentMut::new());
    }

    text.parse::<DocumentMut>().map_err(|error| {
        PluginError::Serialization(format!(
            "failed to parse Codex config.toml at {}: {error}",
            path.display()
        ))
    })
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
            if let Some(headers) = mcp_headers(object) {
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
        "http" | "sse" => ["type", "url", "headers", "http_headers"].as_slice(),
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

fn normalize_mcp_spec(spec: &Value) -> Result<Value, PluginError> {
    validate_mcp_spec(spec)?;
    let mut spec = spec.clone();
    if let Some(object) = spec.as_object_mut() {
        let typ = object
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("stdio")
            .to_string();
        object.insert("type".into(), json!(typ));
    }
    Ok(spec)
}

fn validate_mcp_spec(spec: &Value) -> Result<(), PluginError> {
    let object = spec
        .as_object()
        .ok_or_else(|| PluginError::Serialization("MCP spec must be an object".into()))?;
    let typ = object
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("stdio");
    match typ {
        "stdio" => {
            let command = object.get("command").and_then(Value::as_str).unwrap_or("");
            if command.trim().is_empty() {
                return Err(PluginError::Custom(
                    "stdio MCP spec requires command".into(),
                ));
            }
            if let Some(args) = object.get("args") {
                let all_strings = args
                    .as_array()
                    .map(|values| values.iter().all(Value::is_string))
                    .unwrap_or(false);
                if !all_strings {
                    return Err(PluginError::Custom(
                        "stdio MCP spec args must be an array of strings".into(),
                    ));
                }
            }
        }
        "http" | "sse" => {
            let url = object.get("url").and_then(Value::as_str).unwrap_or("");
            if url.trim().is_empty() {
                return Err(PluginError::Custom("http/sse MCP spec requires url".into()));
            }
        }
        _ => return Err(PluginError::Custom(format!("unsupported MCP type: {typ}"))),
    }
    Ok(())
}

fn sanitize_mcp_id(id: &str) -> Result<String, PluginError> {
    let id = sanitize_provider_id(id);
    if id.is_empty() {
        return Err(PluginError::Custom("MCP id is required".into()));
    }
    Ok(id)
}

fn normalize_stdio_command_for_windows(spec: &mut Value) {
    wrap_stdio_command(spec, cfg!(windows));
}

fn wrap_stdio_command(spec: &mut Value, windows: bool) {
    if !windows {
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

fn normalize_headers_for_claude(spec: &mut Value) {
    let Some(object) = spec.as_object_mut() else {
        return;
    };
    if object.contains_key("headers") {
        return;
    }
    if let Some(headers) = object.get("http_headers").cloned() {
        object.insert("headers".into(), headers);
    }
}

fn normalize_headers_for_codex(spec: &mut Value) {
    let Some(object) = spec.as_object_mut() else {
        return;
    };
    if object.contains_key("http_headers") {
        return;
    }
    if let Some(headers) = object.get("headers").cloned() {
        object.insert("http_headers".into(), headers);
    }
}

fn mcp_headers(object: &Map<String, Value>) -> Option<&Map<String, Value>> {
    object
        .get("http_headers")
        .or_else(|| object.get("headers"))
        .and_then(Value::as_object)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn windows_wrapper_handles_known_stdio_commands() {
        let mut spec = json!({
            "type": "stdio",
            "command": "npx",
            "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
        });

        wrap_stdio_command(&mut spec, true);

        assert_eq!(spec.get("command").and_then(Value::as_str), Some("cmd"));
        assert_eq!(
            spec.get("args").and_then(Value::as_array).cloned(),
            Some(vec![
                json!("/c"),
                json!("npx"),
                json!("-y"),
                json!("@modelcontextprotocol/server-filesystem"),
                json!(".")
            ])
        );
    }

    #[test]
    fn codex_http_mcp_uses_http_headers_alias() {
        let spec = json!({
            "type": "http",
            "url": "https://mcp.example.com",
            "headers": { "Authorization": "Bearer token" }
        });

        let table = json_mcp_to_toml(&spec).expect("valid mcp");

        assert_eq!(
            table.get("url").and_then(|item| item.as_str()),
            Some("https://mcp.example.com")
        );
        assert!(table.get("http_headers").is_some());
        assert!(table.get("headers").is_none());
    }

    #[test]
    fn codex_mcp_preserves_existing_provider_config_and_removes_legacy_mcp_servers() {
        let temp_dir = std::env::temp_dir().join(format!(
            "switchboard-mcp-test-{}",
            crate::fs_util::timestamp_id()
        ));
        fs::create_dir_all(&temp_dir).expect("temp dir");
        let config_path = temp_dir.join("config.toml");
        fs::write(
            &config_path,
            r#"model_provider = "switchboard"
model = "gpt-5.4"

[model_providers.switchboard]
name = "Switchboard"
base_url = "https://api.example.com/v1"
wire_api = "responses"

[mcp.servers.old]
command = "old"
"#,
        )
        .expect("write config");

        install_codex_mcp(
            &config_path,
            "context7",
            &json!({
                "type": "stdio",
                "command": "npx",
                "args": ["-y", "@upstash/context7-mcp"]
            }),
        )
        .expect("install mcp");

        let doc: DocumentMut = fs::read_to_string(&config_path)
            .expect("read config")
            .parse()
            .expect("valid toml");
        assert_eq!(
            doc.get("model_provider").and_then(|item| item.as_str()),
            Some("switchboard")
        );
        assert!(doc
            .get("model_providers")
            .and_then(|item| item.as_table())
            .is_some());
        assert!(doc
            .get("mcp_servers")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("context7"))
            .is_some());
        assert!(doc
            .get("mcp")
            .and_then(|item| item.as_table())
            .and_then(|table| table.get("servers"))
            .is_none());

        let _ = fs::remove_dir_all(temp_dir);
    }
}
