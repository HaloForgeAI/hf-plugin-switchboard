use crate::fs_util::{atomic_write, display_path, pathbufs_to_strings};
use crate::paths::SwitchboardPaths;
use crate::types::{CodexSessionAudit, CodexSessionProviderCount};
use chrono::DateTime;
use hf_plugin_api::PluginError;
use rusqlite::{params_from_iter, types::Value as SqlValue, Connection, OpenFlags};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};
use toml_edit::DocumentMut;

const DEFAULT_CODEX_PROVIDER: &str = "openai";

#[derive(Debug)]
pub struct CodexSessionRepairPlan {
    pub target_provider: String,
    pub session_paths: Vec<PathBuf>,
    pub index_path: Option<PathBuf>,
    pub index_entries: Vec<SessionIndexEntry>,
    pub state_database_path: Option<PathBuf>,
    pub backup_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct CodexSessionRepairApply {
    pub audit: CodexSessionAudit,
    pub changed_paths: Vec<PathBuf>,
    pub session_files_changed: usize,
    pub index_entries_written: usize,
    pub state_threads_updated: usize,
    pub warnings: Vec<String>,
}

struct SessionScan {
    audit: CodexSessionAudit,
    mismatched_session_paths: Vec<PathBuf>,
    index_path: Option<PathBuf>,
    index_entries: Vec<SessionIndexEntry>,
    state_database_path: Option<PathBuf>,
    state_threads_to_update: usize,
}

#[derive(Clone, Debug)]
pub struct SessionIndexEntry {
    id: String,
    thread_name: String,
    updated_at: String,
    sort_epoch: i64,
    session_file: PathBuf,
    archived: bool,
    source: String,
    cwd: String,
    created_epoch: i64,
    updated_epoch: i64,
    first_user_message: String,
    sandbox_policy: String,
    approval_mode: String,
    cli_version: String,
    model: Option<String>,
    reasoning_effort: Option<String>,
}

struct SessionInfo {
    id: String,
    provider: String,
    missing_provider: bool,
    archived: bool,
    kind: SessionKind,
    source: String,
    cwd: String,
    timestamp: String,
    updated_at: String,
    title: String,
    cli_version: String,
    model: Option<String>,
    reasoning_effort: Option<String>,
    sandbox_policy: String,
    approval_mode: String,
    path: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SessionKind {
    Desktop,
    Cli,
    Unknown,
}

struct IndexStats {
    unique_entries: usize,
    duplicate_entries: usize,
    invalid_lines: usize,
    missing_sessions: usize,
    entries: BTreeMap<String, ExistingIndexEntry>,
    warnings: Vec<String>,
}

#[derive(Clone)]
struct ExistingIndexEntry {
    thread_name: String,
    updated_at: String,
}

struct StateDbStats {
    path: Option<PathBuf>,
    thread_rows: usize,
    current_provider_rows: usize,
    other_provider_rows: usize,
    missing_provider_rows: usize,
    missing_session_rows: usize,
    threads_to_update: usize,
    warnings: Vec<String>,
}

pub fn audit_codex_sessions(paths: &SwitchboardPaths) -> Result<CodexSessionAudit, PluginError> {
    Ok(scan_codex_sessions(paths, false)?.audit)
}

pub fn plan_codex_session_provider_repair(
    paths: &SwitchboardPaths,
    include_archived: bool,
) -> Result<CodexSessionRepairPlan, PluginError> {
    let scan = scan_codex_sessions(paths, include_archived)?;
    let mut backup_paths = scan.mismatched_session_paths.clone();
    if let Some(path) = &scan.index_path {
        backup_paths.push(path.clone());
    }
    if scan.state_threads_to_update > 0 {
        if let Some(path) = &scan.state_database_path {
            backup_paths.push(path.clone());
        }
    }
    backup_paths.sort();
    backup_paths.dedup();

    Ok(CodexSessionRepairPlan {
        target_provider: scan.audit.current_provider,
        session_paths: scan.mismatched_session_paths,
        index_path: scan.index_path,
        index_entries: scan.index_entries,
        state_database_path: scan.state_database_path,
        backup_paths,
    })
}

pub fn apply_codex_session_provider_repair(
    paths: &SwitchboardPaths,
    plan: CodexSessionRepairPlan,
) -> Result<CodexSessionRepairApply, PluginError> {
    let mut changed_paths = Vec::new();
    let mut warnings = Vec::new();
    let mut session_files_changed = 0;

    for path in &plan.session_paths {
        match retag_session_file(path, &plan.target_provider) {
            Ok(true) => {
                changed_paths.push(path.clone());
                session_files_changed += 1;
            }
            Ok(false) => {}
            Err(error) => warnings.push(format!("{}: {error:?}", path.display())),
        }
    }

    let mut index_entries_written = 0;
    if let Some(path) = &plan.index_path {
        match rebuild_session_index(path, &plan.index_entries) {
            Ok(count) => {
                changed_paths.push(path.clone());
                index_entries_written = count;
            }
            Err(error) => warnings.push(format!("{}: {error:?}", path.display())),
        }
    }

    let mut state_threads_updated = 0;
    if let Some(path) = &plan.state_database_path {
        match repair_state_database_threads(path, &plan.target_provider, &plan.index_entries) {
            Ok(count) => {
                if count > 0 {
                    changed_paths.push(path.clone());
                    state_threads_updated = count;
                }
            }
            Err(error) => warnings.push(format!("{}: {error:?}", path.display())),
        }
    }

    let mut audit = audit_codex_sessions(paths)?;
    audit.warnings.extend(warnings.clone());
    Ok(CodexSessionRepairApply {
        audit,
        changed_paths,
        session_files_changed,
        index_entries_written,
        state_threads_updated,
        warnings,
    })
}

fn scan_codex_sessions(
    paths: &SwitchboardPaths,
    include_archived: bool,
) -> Result<SessionScan, PluginError> {
    let mut warnings = Vec::new();
    let current_provider = detect_current_provider(&paths.codex_config_path, &mut warnings);
    let sessions_dir = paths.codex_dir.join("sessions");
    let archived_dir = paths.codex_dir.join("archived_sessions");
    let active_files = collect_jsonl_files(&sessions_dir);
    let archived_files = collect_jsonl_files(&archived_dir);
    let mut provider_counts = BTreeMap::<String, usize>::new();
    let mut sessions_missing_provider = 0;
    let mut hidden_session_candidates = 0;
    let mut mismatched_session_paths = Vec::new();
    let mut session_infos = Vec::new();

    for (path, archived) in active_files
        .iter()
        .map(|path| (path, false))
        .chain(archived_files.iter().map(|path| (path, true)))
    {
        match read_session_info(path, archived) {
            Ok(Some(info)) => {
                *provider_counts.entry(info.provider.clone()).or_insert(0) += 1;
                if info.missing_provider {
                    sessions_missing_provider += 1;
                }
                if info.provider != current_provider {
                    if !archived {
                        hidden_session_candidates += 1;
                    }
                    if include_archived || !archived {
                        mismatched_session_paths.push(path.clone());
                    }
                } else if info.missing_provider && (include_archived || !archived) {
                    mismatched_session_paths.push(path.clone());
                }
                session_infos.push(info);
            }
            Ok(None) => warnings.push(format!("No session_meta found in {}", path.display())),
            Err(error) => warnings.push(format!("Failed to inspect {}: {error:?}", path.display())),
        }
    }

    let index_path = paths.codex_dir.join("session_index.jsonl");
    let index_stats = inspect_session_index(&index_path, &session_infos);
    warnings.extend(index_stats.warnings.clone());
    let index_entries = build_session_index_entries(&session_infos, &index_stats.entries);
    let index_needs_rebuild = index_stats.duplicate_entries > 0
        || index_stats.invalid_lines > 0
        || index_stats.missing_sessions > 0;
    let state_stats = inspect_state_database(&paths.codex_dir, &current_provider, &index_entries);
    warnings.extend(state_stats.warnings);

    let provider_counts = provider_counts
        .into_iter()
        .map(|(provider, count)| CodexSessionProviderCount {
            current: provider == current_provider,
            provider,
            count,
        })
        .collect::<Vec<_>>();

    let audit = CodexSessionAudit {
        codex_home: display_path(&paths.codex_dir),
        current_provider,
        session_files: active_files.len(),
        archived_session_files: archived_files.len(),
        sessions_missing_provider,
        hidden_session_candidates,
        indexed_sessions: index_stats.unique_entries,
        index_duplicate_entries: index_stats.duplicate_entries,
        index_missing_sessions: index_stats.missing_sessions,
        state_database_path: state_stats.path.as_deref().map(display_path),
        state_thread_rows: state_stats.thread_rows,
        state_thread_current_provider: state_stats.current_provider_rows,
        state_thread_other_provider: state_stats.other_provider_rows,
        state_thread_missing_provider: state_stats.missing_provider_rows,
        state_thread_missing_sessions: state_stats.missing_session_rows,
        provider_counts,
        warnings,
    };

    Ok(SessionScan {
        audit,
        mismatched_session_paths,
        index_path: index_needs_rebuild.then_some(index_path),
        index_entries,
        state_database_path: state_stats.path,
        state_threads_to_update: state_stats.threads_to_update,
    })
}

fn detect_current_provider(config_path: &Path, warnings: &mut Vec<String>) -> String {
    let Ok(text) = fs::read_to_string(config_path) else {
        return DEFAULT_CODEX_PROVIDER.to_string();
    };
    if text.trim().is_empty() {
        return DEFAULT_CODEX_PROVIDER.to_string();
    }
    match text.parse::<DocumentMut>() {
        Ok(doc) => doc
            .get("model_provider")
            .and_then(|item| item.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_CODEX_PROVIDER)
            .to_string(),
        Err(error) => {
            warnings.push(format!(
                "Failed to parse {}: {error}",
                config_path.display()
            ));
            DEFAULT_CODEX_PROVIDER.to_string()
        }
    }
}

fn collect_jsonl_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().and_then(|value| value.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

fn read_session_info(path: &Path, archived: bool) -> Result<Option<SessionInfo>, PluginError> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let expected_id = session_id_from_filename(path);
    let mut payloads = Vec::new();
    let mut first_prompt = String::new();
    let mut last_timestamp = String::new();
    let mut sandbox_policy = "{}".to_string();
    let mut approval_mode = "on-request".to_string();
    let mut model = None;
    let mut reasoning_effort = None;

    for line in reader.lines() {
        let line = line?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(timestamp) = value.get("timestamp").and_then(Value::as_str) {
            if !timestamp.trim().is_empty() {
                last_timestamp = timestamp.trim().to_string();
            }
        }
        if value.get("type").and_then(Value::as_str) == Some("session_meta") {
            if let Some(payload) = value.get("payload").and_then(Value::as_object) {
                payloads.push(Value::Object(payload.clone()));
            }
            continue;
        }
        if value.get("type").and_then(Value::as_str) == Some("turn_context") {
            if let Some(payload) = value.get("payload").and_then(Value::as_object) {
                sandbox_policy = payload
                    .get("sandbox_policy")
                    .map(|value| serde_json::to_string(value).unwrap_or_else(|_| "{}".into()))
                    .unwrap_or_else(|| "{}".into());
                approval_mode = payload
                    .get("approval_policy")
                    .and_then(Value::as_str)
                    .unwrap_or("on-request")
                    .to_string();
                model = payload
                    .get("model")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                reasoning_effort = payload
                    .get("effort")
                    .and_then(Value::as_str)
                    .map(str::to_string);
            }
        }
        if first_prompt.is_empty() {
            first_prompt = first_user_prompt(&value);
        }
    }

    let Some(payload) = select_effective_payload(&payloads, expected_id.as_deref()) else {
        return Ok(None);
    };
    let Some(payload_object) = payload.as_object() else {
        return Ok(None);
    };
    let id = payload_object
        .get("id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .or(expected_id)
        .unwrap_or_else(|| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .unwrap_or("")
                .to_string()
        });
    if id.trim().is_empty() {
        return Ok(None);
    }
    let provider = payload_object
        .get("model_provider")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let missing_provider = provider.is_none();
    let source = string_field(payload_object.get("source"));
    let originator = string_field(payload_object.get("originator"));
    let cwd = string_field(payload_object.get("cwd"));
    let timestamp = string_field(payload_object.get("timestamp"));
    let updated_at = normalize_iso(&last_timestamp)
        .or_else(|| normalize_iso(&timestamp))
        .or_else(|| iso_from_rollout_filename(path))
        .unwrap_or_else(|| "1970-01-01T00:00:00Z".to_string());
    let title = build_session_title(&first_prompt, path, &cwd, &updated_at);

    Ok(Some(SessionInfo {
        id,
        provider: provider.unwrap_or_else(|| DEFAULT_CODEX_PROVIDER.to_string()),
        missing_provider,
        archived,
        kind: classify_session_kind(&source, &originator),
        source,
        cwd,
        timestamp,
        updated_at,
        title,
        cli_version: string_field(payload_object.get("cli_version")),
        model,
        reasoning_effort,
        sandbox_policy,
        approval_mode,
        path: path.to_path_buf(),
    }))
}

fn select_effective_payload(payloads: &[Value], expected_id: Option<&str>) -> Option<Value> {
    if let Some(expected_id) = expected_id {
        for payload in payloads.iter().rev() {
            let payload_id = payload
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("");
            if payload_id.eq_ignore_ascii_case(expected_id) {
                return Some(payload.clone());
            }
        }
    }
    payloads.first().cloned()
}

fn string_field(value: Option<&Value>) -> String {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
        .to_string()
}

fn classify_session_kind(source: &str, originator: &str) -> SessionKind {
    if source == "vscode" || originator.contains("Desktop") {
        return SessionKind::Desktop;
    }
    if source == "cli" || originator == "codex_cli_rs" || originator.starts_with("codex_cli") {
        return SessionKind::Cli;
    }
    SessionKind::Unknown
}

fn first_user_prompt(value: &Value) -> String {
    let payload = value.get("payload").unwrap_or(value);
    let candidate = match value.get("type").and_then(Value::as_str) {
        Some("response_item") | Some("message")
            if payload.get("role").and_then(Value::as_str) == Some("user") =>
        {
            first_text_fragment(payload.get("content").or_else(|| payload.get("text")))
        }
        Some("event_msg")
            if payload.get("type").and_then(Value::as_str) == Some("user_message") =>
        {
            first_text_fragment(payload.get("message").or_else(|| payload.get("text")))
        }
        _ => String::new(),
    };
    summarize_session_prompt(&candidate)
}

fn first_text_fragment(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => normalize_session_text(text),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| first_text_fragment(Some(item)))
            .find(|text| !text.is_empty())
            .unwrap_or_default(),
        Some(Value::Object(map)) => ["text", "message", "content"]
            .iter()
            .filter_map(|key| map.get(*key))
            .map(|item| first_text_fragment(Some(item)))
            .find(|text| !text.is_empty())
            .unwrap_or_default(),
        _ => String::new(),
    }
}

fn normalize_session_text(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn summarize_session_prompt(text: &str) -> String {
    let normalized = normalize_session_text(text);
    if normalized.is_empty() {
        return normalized;
    }
    let lowered = normalized.to_ascii_lowercase();
    for marker in [
        "## my request for codex:",
        "## my request for cursor:",
        "## my request for chatgpt:",
        "## task",
    ] {
        if let Some(index) = lowered.find(marker) {
            let summary = normalized[index + marker.len()..].trim();
            return if summary.is_empty() {
                normalized
            } else {
                summary.to_string()
            };
        }
    }
    normalized
}

fn build_session_title(prompt: &str, path: &Path, cwd: &str, updated_at: &str) -> String {
    let prompt = summarize_session_prompt(prompt);
    if !is_placeholder_thread_name(&prompt, "") {
        return prompt;
    }
    let workspace = workspace_name_from_cwd(cwd);
    if !workspace.is_empty() {
        return workspace;
    }
    if let Some(date) = updated_at
        .split('T')
        .next()
        .filter(|value| !value.is_empty())
    {
        return format!("Codex session {date}");
    }
    path.file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("Codex session")
        .to_string()
}

fn workspace_name_from_cwd(cwd: &str) -> String {
    cwd.trim()
        .trim_end_matches(['\\', '/'])
        .rsplit(['\\', '/'])
        .next()
        .unwrap_or("")
        .to_string()
}

fn is_placeholder_thread_name(thread_name: &str, session_id: &str) -> bool {
    let normalized = normalize_session_text(thread_name);
    if normalized.is_empty() {
        return true;
    }
    if !session_id.is_empty() && normalized == session_id {
        return true;
    }
    let lowered = normalized.to_ascii_lowercase();
    lowered.starts_with("<environment_context>")
        || lowered.starts_with("<permissions instructions>")
        || lowered.starts_with("<app-context>")
        || lowered.starts_with("# agents.md instructions")
        || lowered.starts_with("# context from my ide setup:")
}

fn session_id_from_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let name = name.strip_prefix("rollout-")?.strip_suffix(".jsonl")?;
    if name.len() < 36 {
        return None;
    }
    let id = &name[name.len() - 36..];
    if id.len() == 36 {
        Some(id.to_string())
    } else {
        None
    }
}

fn iso_from_rollout_filename(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    let rest = name.strip_prefix("rollout-")?;
    let timestamp = rest.get(0..19)?;
    if timestamp.len() != 19 {
        return None;
    }
    Some(format!(
        "{}:{}:{}Z",
        timestamp.get(0..13)?,
        timestamp.get(14..16)?,
        timestamp.get(17..19)?
    ))
}

fn normalize_iso(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.contains('T') && trimmed.len() >= 20 {
        Some(trimmed.to_string())
    } else {
        None
    }
}

fn iso_to_epoch(value: &str) -> i64 {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.timestamp())
        .unwrap_or(0)
}

fn inspect_session_index(path: &Path, session_infos: &[SessionInfo]) -> IndexStats {
    let mut entries = BTreeMap::new();
    let mut duplicate_entries = 0;
    let mut invalid_lines = 0;
    let mut warnings = Vec::new();

    if let Ok(file) = fs::File::open(path) {
        for line in BufReader::new(file).lines().map_while(Result::ok) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                invalid_lines += 1;
                continue;
            };
            let Some(id) = value
                .get("id")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
            else {
                invalid_lines += 1;
                continue;
            };
            if entries.contains_key(id) {
                duplicate_entries += 1;
            }
            entries.insert(
                id.to_string(),
                ExistingIndexEntry {
                    thread_name: value
                        .get("thread_name")
                        .and_then(Value::as_str)
                        .unwrap_or(id)
                        .to_string(),
                    updated_at: value
                        .get("updated_at")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                },
            );
        }
    }

    if invalid_lines > 0 {
        warnings.push(format!(
            "session_index.jsonl has {invalid_lines} malformed line(s); repair will rebuild the index"
        ));
    }

    let expected_ids = session_infos
        .iter()
        .filter(|info| info.kind == SessionKind::Desktop && !info.archived)
        .map(|info| info.id.as_str())
        .collect::<BTreeSet<_>>();
    let missing_sessions = expected_ids
        .iter()
        .filter(|id| !entries.contains_key(**id))
        .count();

    IndexStats {
        unique_entries: entries.len(),
        duplicate_entries,
        invalid_lines,
        missing_sessions,
        entries,
        warnings,
    }
}

fn build_session_index_entries(
    session_infos: &[SessionInfo],
    existing_index: &BTreeMap<String, ExistingIndexEntry>,
) -> Vec<SessionIndexEntry> {
    session_infos
        .iter()
        .filter(|info| info.kind == SessionKind::Desktop)
        .map(|info| {
            let existing = existing_index.get(&info.id);
            let existing_title = existing
                .map(|entry| entry.thread_name.as_str())
                .unwrap_or("");
            let thread_name = if is_placeholder_thread_name(existing_title, &info.id) {
                info.title.clone()
            } else {
                existing_title.to_string()
            };
            let updated_at = existing
                .and_then(|entry| normalize_iso(&entry.updated_at))
                .unwrap_or_else(|| info.updated_at.clone());
            let created_at = normalize_iso(&info.timestamp).unwrap_or_else(|| updated_at.clone());
            SessionIndexEntry {
                id: info.id.clone(),
                thread_name,
                updated_at: updated_at.clone(),
                sort_epoch: iso_to_epoch(&updated_at),
                session_file: info.path.clone(),
                archived: info.archived,
                source: if info.source.is_empty() {
                    "vscode".to_string()
                } else {
                    info.source.clone()
                },
                cwd: info.cwd.clone(),
                created_epoch: iso_to_epoch(&created_at),
                updated_epoch: iso_to_epoch(&updated_at),
                first_user_message: info.title.clone(),
                sandbox_policy: info.sandbox_policy.clone(),
                approval_mode: info.approval_mode.clone(),
                cli_version: info.cli_version.clone(),
                model: info.model.clone(),
                reasoning_effort: info.reasoning_effort.clone(),
            }
        })
        .collect()
}

fn rebuild_session_index(path: &Path, updates: &[SessionIndexEntry]) -> Result<usize, PluginError> {
    let existing = inspect_session_index(path, &[]);
    let mut entries = existing
        .entries
        .into_iter()
        .map(|(id, entry)| {
            let sort_epoch = iso_to_epoch(&entry.updated_at);
            (
                id.clone(),
                IndexOutputEntry {
                    id,
                    thread_name: entry.thread_name,
                    updated_at: entry.updated_at,
                    sort_epoch,
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    for update in updates {
        entries.insert(
            update.id.clone(),
            IndexOutputEntry {
                id: update.id.clone(),
                thread_name: update.thread_name.clone(),
                updated_at: update.updated_at.clone(),
                sort_epoch: update.sort_epoch,
            },
        );
    }

    let mut output = entries.into_values().collect::<Vec<_>>();
    output.sort_by(|a, b| b.sort_epoch.cmp(&a.sort_epoch).then(a.id.cmp(&b.id)));

    let mut text = String::new();
    for entry in &output {
        let value = serde_json::json!({
            "id": entry.id,
            "thread_name": entry.thread_name,
            "updated_at": entry.updated_at,
        });
        text.push_str(
            &serde_json::to_string(&value)
                .map_err(|error| PluginError::Serialization(error.to_string()))?,
        );
        text.push('\n');
    }
    atomic_write(path, text.as_bytes())?;
    Ok(output.len())
}

struct IndexOutputEntry {
    id: String,
    thread_name: String,
    updated_at: String,
    sort_epoch: i64,
}

fn inspect_state_database(
    codex_dir: &Path,
    current_provider: &str,
    index_entries: &[SessionIndexEntry],
) -> StateDbStats {
    let Some(path) = latest_state_database(codex_dir) else {
        return StateDbStats {
            path: None,
            thread_rows: 0,
            current_provider_rows: 0,
            other_provider_rows: 0,
            missing_provider_rows: 0,
            missing_session_rows: 0,
            threads_to_update: 0,
            warnings: Vec::new(),
        };
    };

    let mut stats = StateDbStats {
        path: Some(path.clone()),
        thread_rows: 0,
        current_provider_rows: 0,
        other_provider_rows: 0,
        missing_provider_rows: 0,
        missing_session_rows: 0,
        threads_to_update: 0,
        warnings: Vec::new(),
    };

    let database = match open_database(&path, false) {
        Ok(database) => database,
        Err(error) => {
            stats.warnings.push(format!("{error:?}"));
            return stats;
        }
    };

    if !table_exists(&database, "threads").unwrap_or(false) {
        stats
            .warnings
            .push(format!("threads table not found in {}", path.display()));
        return stats;
    }
    let has_model_provider =
        table_has_column(&database, "threads", "model_provider").unwrap_or(false);
    let has_id = table_has_column(&database, "threads", "id").unwrap_or(false);
    if !has_model_provider {
        stats.warnings.push(format!(
            "threads.model_provider column not found in {}",
            path.display()
        ));
    } else if let Ok(mut statement) = database.prepare(
        "select coalesce(model_provider, '') as provider, count(*) from threads group by provider",
    ) {
        if let Ok(rows) = statement.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        }) {
            for row in rows.flatten() {
                let provider = row.0.trim().to_string();
                let count = usize::try_from(row.1).unwrap_or(0);
                stats.thread_rows += count;
                if provider.is_empty() {
                    stats.missing_provider_rows += count;
                } else if provider == current_provider {
                    stats.current_provider_rows += count;
                } else {
                    stats.other_provider_rows += count;
                }
            }
        }
    }

    if has_id {
        let mut existing_ids = BTreeSet::new();
        if let Ok(mut statement) = database.prepare("select id from threads") {
            if let Ok(rows) = statement.query_map([], |row| row.get::<_, String>(0)) {
                for id in rows.flatten() {
                    existing_ids.insert(id);
                }
            }
        }
        stats.missing_session_rows = index_entries
            .iter()
            .filter(|entry| !entry.archived)
            .filter(|entry| !existing_ids.contains(&entry.id))
            .count();
    }

    stats.threads_to_update =
        stats.other_provider_rows + stats.missing_provider_rows + stats.missing_session_rows;
    stats
}

fn latest_state_database(codex_dir: &Path) -> Option<PathBuf> {
    let entries = fs::read_dir(codex_dir).ok()?;
    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };
        let Some(number) = state_database_number(name) else {
            continue;
        };
        let modified = path
            .metadata()
            .and_then(|metadata| metadata.modified())
            .unwrap_or(UNIX_EPOCH)
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        candidates.push((number, modified, name.to_string(), path));
    }
    candidates.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    candidates.pop().map(|(_, _, _, path)| path)
}

fn state_database_number(name: &str) -> Option<u64> {
    let rest = name.strip_prefix("state_")?.strip_suffix(".sqlite")?;
    rest.parse::<u64>().ok()
}

fn retag_session_file(path: &Path, provider: &str) -> Result<bool, PluginError> {
    let info = read_session_info(path, false)?;
    let Some(info) = info else {
        return Ok(false);
    };
    let text = fs::read_to_string(path)?;
    let mut changed = false;
    let mut output = String::new();
    let mut fallback_updated = false;

    for segment in split_lines_preserve_endings(&text) {
        let line = segment.trim_end_matches(['\r', '\n']);
        let ending = &segment[line.len()..];
        let parsed = serde_json::from_str::<Value>(line);
        match parsed {
            Ok(mut value) if value.get("type").and_then(Value::as_str) == Some("session_meta") => {
                if let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) {
                    let payload_id = payload
                        .get("id")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or("");
                    let should_patch =
                        payload_id == info.id || (payload_id.is_empty() && !fallback_updated);
                    if should_patch {
                        let current = payload
                            .get("model_provider")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            .unwrap_or(DEFAULT_CODEX_PROVIDER);
                        if current != provider || !payload.contains_key("model_provider") {
                            payload.insert(
                                "model_provider".into(),
                                Value::String(provider.to_string()),
                            );
                            changed = true;
                            fallback_updated = true;
                            output.push_str(
                                &serde_json::to_string(&value).map_err(|error| {
                                    PluginError::Serialization(error.to_string())
                                })?,
                            );
                            output.push_str(ending);
                            continue;
                        }
                    }
                }
                output.push_str(segment);
            }
            _ => output.push_str(segment),
        }
    }

    if changed {
        atomic_write(path, output.as_bytes())?;
    }
    Ok(changed)
}

fn split_lines_preserve_endings(text: &str) -> Vec<&str> {
    let mut lines = text.split_inclusive('\n').collect::<Vec<_>>();
    if text.is_empty() || text.ends_with('\n') {
        return lines;
    }
    if lines.is_empty() {
        lines.push(text);
    }
    lines
}

fn repair_state_database_threads(
    path: &Path,
    provider: &str,
    entries: &[SessionIndexEntry],
) -> Result<usize, PluginError> {
    let database = open_database(path, true)?;
    if !table_exists(&database, "threads")? {
        return Ok(0);
    }
    let columns = table_columns(&database, "threads")?;
    let mut changed = 0;
    if columns.contains("model_provider") {
        changed += database
            .execute(
                "update threads set model_provider = ?1 where model_provider is null or model_provider = '' or model_provider <> ?1",
                [provider],
            )
            .map_err(|error| PluginError::Io(format!("failed to update {}: {error}", path.display())))?;
    }
    if !columns.contains("id") {
        return Ok(changed);
    }

    for entry in entries.iter().filter(|entry| !entry.archived) {
        let mut data = Vec::<(&str, SqlValue)>::new();
        push_sql(&mut data, &columns, "id", SqlValue::Text(entry.id.clone()));
        push_sql(
            &mut data,
            &columns,
            "rollout_path",
            SqlValue::Text(display_path(&entry.session_file)),
        );
        push_sql(
            &mut data,
            &columns,
            "created_at",
            SqlValue::Integer(entry.created_epoch),
        );
        push_sql(
            &mut data,
            &columns,
            "updated_at",
            SqlValue::Integer(entry.updated_epoch),
        );
        push_sql(
            &mut data,
            &columns,
            "source",
            SqlValue::Text(entry.source.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "model_provider",
            SqlValue::Text(provider.to_string()),
        );
        push_sql(
            &mut data,
            &columns,
            "cwd",
            SqlValue::Text(entry.cwd.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "title",
            SqlValue::Text(entry.thread_name.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "sandbox_policy",
            SqlValue::Text(entry.sandbox_policy.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "approval_mode",
            SqlValue::Text(entry.approval_mode.clone()),
        );
        push_sql(&mut data, &columns, "tokens_used", SqlValue::Integer(0));
        push_sql(&mut data, &columns, "has_user_event", SqlValue::Integer(1));
        push_sql(&mut data, &columns, "archived", SqlValue::Integer(0));
        push_sql(&mut data, &columns, "archived_at", SqlValue::Null);
        push_sql(
            &mut data,
            &columns,
            "cli_version",
            SqlValue::Text(entry.cli_version.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "first_user_message",
            SqlValue::Text(entry.first_user_message.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "memory_mode",
            SqlValue::Text("enabled".to_string()),
        );
        push_sql(
            &mut data,
            &columns,
            "model",
            option_sql_text(entry.model.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "reasoning_effort",
            option_sql_text(entry.reasoning_effort.clone()),
        );
        push_sql(
            &mut data,
            &columns,
            "preview",
            SqlValue::Text(entry.first_user_message.clone()),
        );

        let insert_cols = data.iter().map(|(name, _)| *name).collect::<Vec<_>>();
        if insert_cols.len() <= 1 {
            continue;
        }
        let placeholders = std::iter::repeat("?")
            .take(insert_cols.len())
            .collect::<Vec<_>>()
            .join(", ");
        let col_list = insert_cols.join(", ");
        let update_sql = insert_cols
            .iter()
            .filter(|name| **name != "id")
            .map(|name| format!("{name}=excluded.{name}"))
            .collect::<Vec<_>>()
            .join(", ");
        let values = data.into_iter().map(|(_, value)| value).collect::<Vec<_>>();
        let sql = format!(
            "insert into threads ({col_list}) values ({placeholders}) on conflict(id) do update set {update_sql}"
        );
        database
            .execute(&sql, params_from_iter(values.iter()))
            .map_err(|error| PluginError::Io(format!("failed to upsert {}: {error}", entry.id)))?;
        changed += 1;
    }

    Ok(changed)
}

fn push_sql(
    data: &mut Vec<(&'static str, SqlValue)>,
    columns: &BTreeSet<String>,
    name: &'static str,
    value: SqlValue,
) {
    if columns.contains(name) {
        data.push((name, value));
    }
}

fn option_sql_text(value: Option<String>) -> SqlValue {
    value
        .filter(|value| !value.trim().is_empty())
        .map(SqlValue::Text)
        .unwrap_or(SqlValue::Null)
}

fn open_database(path: &Path, write: bool) -> Result<Connection, PluginError> {
    let flags = if write {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let database = Connection::open_with_flags(path, flags)
        .map_err(|error| PluginError::Io(format!("failed to open {}: {error}", path.display())))?;
    database
        .busy_timeout(Duration::from_secs(5))
        .map_err(|error| {
            PluginError::Io(format!(
                "failed to configure SQLite busy timeout for {}: {error}",
                path.display()
            ))
        })?;
    Ok(database)
}

fn table_exists(database: &Connection, table_name: &str) -> Result<bool, PluginError> {
    let mut statement = database
        .prepare("select 1 from sqlite_master where type = 'table' and name = ?1 limit 1")
        .map_err(|error| PluginError::Io(format!("failed to inspect SQLite schema: {error}")))?;
    let mut rows = statement
        .query([table_name])
        .map_err(|error| PluginError::Io(format!("failed to query SQLite schema: {error}")))?;
    rows.next()
        .map(|row| row.is_some())
        .map_err(|error| PluginError::Io(format!("failed to read SQLite schema row: {error}")))
}

fn table_has_column(
    database: &Connection,
    table_name: &str,
    column_name: &str,
) -> Result<bool, PluginError> {
    Ok(table_columns(database, table_name)?.contains(column_name))
}

fn table_columns(database: &Connection, table_name: &str) -> Result<BTreeSet<String>, PluginError> {
    let mut statement = database
        .prepare(&format!("pragma table_info({table_name})"))
        .map_err(|error| PluginError::Io(format!("failed to inspect {table_name}: {error}")))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| PluginError::Io(format!("failed to query {table_name}: {error}")))?;
    let mut columns = BTreeSet::new();
    for row in rows {
        columns.insert(
            row.map_err(|error| PluginError::Io(format!("failed to read {table_name}: {error}")))?,
        );
    }
    Ok(columns)
}

pub fn changed_paths_for_result(paths: &[PathBuf]) -> Vec<String> {
    let mut unique = BTreeSet::new();
    let unique_paths = paths
        .iter()
        .filter(|path| unique.insert(display_path(path)))
        .cloned()
        .collect::<Vec<_>>();
    pathbufs_to_strings(&unique_paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_util::timestamp_id;

    fn test_paths(codex_dir: PathBuf) -> SwitchboardPaths {
        let home = codex_dir.parent().expect("codex parent").join("home");
        SwitchboardPaths {
            home: home.clone(),
            claude_settings_path: home.join(".claude/settings.json"),
            claude_config_path: home.join(".claude/config.json"),
            claude_mcp_path: home.join(".claude.json"),
            codex_auth_path: codex_dir.join("auth.json"),
            codex_config_path: codex_dir.join("config.toml"),
            codex_dir,
        }
    }

    #[test]
    fn repairs_session_meta_index_and_state_threads_to_current_provider() {
        let root = std::env::temp_dir().join(format!("switchboard-sessions-{}", timestamp_id()));
        let codex_dir = root.join(".codex");
        let session_dir = codex_dir.join("sessions/2026/07/02");
        fs::create_dir_all(&session_dir).expect("create session dir");
        fs::write(codex_dir.join("config.toml"), "model_provider = \"new\"\n")
            .expect("write config");
        fs::write(
            codex_dir.join("session_index.jsonl"),
            "{\"id\":\"stale\",\"thread_name\":\"Old\",\"updated_at\":\"2026-07-01T00:00:00Z\"}\n",
        )
        .expect("write index");
        let session_path = session_dir
            .join("rollout-2026-07-02T12-00-00-11111111-1111-1111-1111-111111111111.jsonl");
        fs::write(
            &session_path,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"11111111-1111-1111-1111-111111111111\",\"source\":\"vscode\",\"originator\":\"Codex Desktop\",\"cwd\":\"G:/Git/demo\",\"model_provider\":\"old\"}}\n{\"timestamp\":\"2026-07-02T12:01:00Z\",\"type\":\"response_item\",\"payload\":{\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello switchboard\"}]}}\n",
        )
        .expect("write session");

        let database_path = codex_dir.join("state_5.sqlite");
        let database = Connection::open(&database_path).expect("create database");
        database
            .execute_batch(
                "create table threads(id text primary key, rollout_path text, model_provider text, title text, cwd text, updated_at integer, created_at integer, source text, first_user_message text, has_user_event integer);
                 insert into threads(id, model_provider) values('a', 'old');
                 insert into threads(id, model_provider) values('b', null);",
            )
            .expect("seed database");
        drop(database);

        let paths = test_paths(codex_dir.clone());
        let audit = audit_codex_sessions(&paths).expect("audit before");
        assert_eq!(audit.current_provider, "new");
        assert_eq!(audit.hidden_session_candidates, 1);
        assert_eq!(audit.index_missing_sessions, 1);
        assert_eq!(audit.state_thread_other_provider, 1);
        assert_eq!(audit.state_thread_missing_provider, 1);
        assert_eq!(audit.state_thread_missing_sessions, 1);

        let plan = plan_codex_session_provider_repair(&paths, false).expect("plan");
        assert_eq!(plan.session_paths, vec![session_path.clone()]);
        assert_eq!(plan.index_path, Some(codex_dir.join("session_index.jsonl")));
        assert_eq!(plan.state_database_path, Some(database_path.clone()));

        let result = apply_codex_session_provider_repair(&paths, plan).expect("repair");
        assert_eq!(result.session_files_changed, 1);
        assert!(result.index_entries_written >= 1);
        assert!(result.state_threads_updated >= 3);
        assert_eq!(result.audit.hidden_session_candidates, 0);
        assert_eq!(result.audit.index_missing_sessions, 0);
        assert_eq!(result.audit.state_thread_other_provider, 0);
        assert_eq!(result.audit.state_thread_missing_provider, 0);
        assert_eq!(result.audit.state_thread_missing_sessions, 0);
        let text = fs::read_to_string(&session_path).expect("read repaired session");
        assert!(text.contains("\"model_provider\":\"new\""));
        let index = fs::read_to_string(codex_dir.join("session_index.jsonl")).expect("read index");
        assert!(index.contains("11111111-1111-1111-1111-111111111111"));
        assert!(index.contains("hello switchboard"));

        let repaired_database = Connection::open(&database_path).expect("open database");
        let count: i64 = repaired_database
            .query_row(
                "select count(*) from threads where id = '11111111-1111-1111-1111-111111111111' and model_provider = 'new'",
                [],
                |row| row.get(0),
            )
            .expect("count repaired row");
        assert_eq!(count, 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn effective_session_meta_prefers_filename_matching_payload() {
        let root = std::env::temp_dir().join(format!("switchboard-sessions-{}", timestamp_id()));
        let codex_dir = root.join(".codex");
        let session_dir = codex_dir.join("sessions/2026/07/02");
        fs::create_dir_all(&session_dir).expect("create session dir");
        fs::write(codex_dir.join("config.toml"), "model_provider = \"new\"\n")
            .expect("write config");
        let session_path = session_dir
            .join("rollout-2026-07-02T12-00-00-22222222-2222-2222-2222-222222222222.jsonl");
        fs::write(
            &session_path,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"parent\",\"source\":\"vscode\",\"model_provider\":\"parent_provider\"}}\n{\"type\":\"session_meta\",\"payload\":{\"id\":\"22222222-2222-2222-2222-222222222222\",\"source\":\"vscode\",\"model_provider\":\"old\"}}\n",
        )
        .expect("write session");

        let paths = test_paths(codex_dir.clone());
        let plan = plan_codex_session_provider_repair(&paths, false).expect("plan");
        apply_codex_session_provider_repair(&paths, plan).expect("repair");
        let text = fs::read_to_string(&session_path).expect("read repaired session");
        assert!(text.contains(
            "\"id\":\"parent\",\"source\":\"vscode\",\"model_provider\":\"parent_provider\""
        ));
        assert!(text.contains("\"id\":\"22222222-2222-2222-2222-222222222222\""));
        assert!(text.contains("\"model_provider\":\"new\""));

        let _ = fs::remove_dir_all(root);
    }
}
