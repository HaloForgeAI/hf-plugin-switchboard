use crate::backup::list_backup_infos;
use crate::fs_util::{display_path, mask_secret, read_json};
use crate::paths::SwitchboardPaths;
use crate::types::{
    PathStatus, SwitchboardStatus, TargetDetail, TargetStatus, CLAUDE_TARGET, CODEX_TARGET,
};
use hf_plugin_api::{PluginContext, PluginError};
use serde_json::Value;
use std::fs;
use std::path::Path;
use toml_edit::DocumentMut;

pub fn read_status(ctx: &dyn PluginContext) -> Result<SwitchboardStatus, PluginError> {
    let paths = SwitchboardPaths::resolve()?;
    Ok(SwitchboardStatus {
        os: std::env::consts::OS.to_string(),
        home_dir: Some(display_path(&paths.home)),
        data_dir: display_path(&ctx.data_dir()),
        targets: vec![claude_status(&paths), codex_status(&paths)],
        backups: list_backup_infos(ctx)?,
    })
}

fn claude_status(paths: &SwitchboardPaths) -> TargetStatus {
    let mut details = Vec::new();
    let summary = read_json(&paths.claude_settings_path)
        .ok()
        .and_then(|value| {
            let env = value.get("env")?.as_object()?;
            let base = env.get("ANTHROPIC_BASE_URL").and_then(Value::as_str);
            let model = env.get("ANTHROPIC_MODEL").and_then(Value::as_str);
            let haiku = env.get("ANTHROPIC_DEFAULT_HAIKU_MODEL").and_then(Value::as_str);
            let sonnet = env.get("ANTHROPIC_DEFAULT_SONNET_MODEL").and_then(Value::as_str);
            let opus = env.get("ANTHROPIC_DEFAULT_OPUS_MODEL").and_then(Value::as_str);
            let token = env
                .get("ANTHROPIC_AUTH_TOKEN")
                .or_else(|| env.get("ANTHROPIC_API_KEY"))
                .and_then(Value::as_str);
            push_detail(&mut details, "Base URL", base, None);
            push_detail(&mut details, "Default model", model, None);
            push_detail(&mut details, "Haiku model", haiku, None);
            push_detail(&mut details, "Sonnet model", sonnet, None);
            push_detail(&mut details, "Opus model", opus, None);
            if let Some(token) = token {
                push_detail(&mut details, "API key", Some(&mask_secret(token)), Some(token));
            }
            Some(summary_parts([base.map(str::to_string), model.map(str::to_string)]))
        })
        .filter(|value| !value.is_empty());

    TargetStatus {
        id: CLAUDE_TARGET.to_string(),
        label: "Claude Code".to_string(),
        configured: paths.claude_settings_path.exists(),
        summary,
        details,
        paths: vec![
            path_status("settings", &paths.claude_settings_path),
            path_status("config", &paths.claude_config_path),
            path_status("mcp", &paths.claude_mcp_path),
        ],
    }
}

fn codex_status(paths: &SwitchboardPaths) -> TargetStatus {
    let mut parts = Vec::new();
    let mut details = Vec::new();
    if let Ok(auth) = read_json(&paths.codex_auth_path) {
        if let Some(key) = auth.get("OPENAI_API_KEY").and_then(Value::as_str) {
            push_detail(&mut details, "API key", Some(&mask_secret(key)), Some(key));
        }
    }
    if let Ok(config) = fs::read_to_string(&paths.codex_config_path) {
        if let Ok(doc) = config.parse::<DocumentMut>() {
            if let Some(provider) = doc.get("model_provider").and_then(|item| item.as_str()) {
                parts.push(provider.to_string());
                push_detail(&mut details, "Provider id", Some(provider), None);
                if let Some(base_url) = doc
                    .get("model_providers")
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get(provider))
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get("base_url"))
                    .and_then(|item| item.as_str())
                {
                    push_detail(&mut details, "Base URL", Some(base_url), None);
                }
                if let Some(name) = doc
                    .get("model_providers")
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get(provider))
                    .and_then(|item| item.as_table())
                    .and_then(|table| table.get("name"))
                    .and_then(|item| item.as_str())
                {
                    push_detail(&mut details, "Provider name", Some(name), None);
                }
            }
            if let Some(model) = doc.get("model").and_then(|item| item.as_str()) {
                parts.push(model.to_string());
                push_detail(&mut details, "Default model", Some(model), None);
            }
            if let Some(reasoning) = doc
                .get("model_reasoning_effort")
                .and_then(|item| item.as_str())
            {
                push_detail(&mut details, "Reasoning", Some(reasoning), None);
            }
        }
    }

    TargetStatus {
        id: CODEX_TARGET.to_string(),
        label: "Codex".to_string(),
        configured: paths.codex_auth_path.exists() || paths.codex_config_path.exists(),
        summary: (!parts.is_empty()).then(|| parts.join(" | ")),
        details,
        paths: vec![
            path_status("auth", &paths.codex_auth_path),
            path_status("config", &paths.codex_config_path),
        ],
    }
}

fn push_detail(
    details: &mut Vec<TargetDetail>,
    label: &str,
    value: Option<&str>,
    secret: Option<&str>,
) {
    let Some(value) = value.map(str::trim).filter(|value| !value.is_empty()) else {
        return;
    };
    details.push(TargetDetail {
        label: label.to_string(),
        value: value.to_string(),
        secret: secret.map(str::to_string),
    });
}

fn path_status(label: &str, path: &Path) -> PathStatus {
    PathStatus {
        label: label.to_string(),
        path: display_path(path),
        exists: path.exists(),
    }
}

fn summary_parts(parts: impl IntoIterator<Item = Option<String>>) -> String {
    parts
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · ")
}
