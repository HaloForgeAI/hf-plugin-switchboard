use crate::fs_util::{atomic_write, display_path, pathbufs_to_strings};
use crate::paths::SwitchboardPaths;
use crate::types::{CodexSessionAudit, CodexSessionProviderCount};
use hf_plugin_api::PluginError;
use rusqlite::{Connection, OpenFlags};
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
    pub state_database_path: Option<PathBuf>,
    pub backup_paths: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct CodexSessionRepairApply {
    pub audit: CodexSessionAudit,
    pub changed_paths: Vec<PathBuf>,
    pub session_files_changed: usize,
    pub state_threads_updated: usize,
    pub warnings: Vec<String>,
}

struct SessionScan {
    audit: CodexSessionAudit,
    mismatched_session_paths: Vec<PathBuf>,
    state_database_path: Option<PathBuf>,
    state_threads_to_update: usize,
}

struct SessionMeta {
    provider: String,
    missing_provider: bool,
}

struct StateDbStats {
    path: Option<PathBuf>,
    thread_rows: usize,
    current_provider_rows: usize,
    other_provider_rows: usize,
    missing_provider_rows: usize,
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

    let mut state_threads_updated = 0;
    if let Some(path) = &plan.state_database_path {
        match retag_state_database_threads(path, &plan.target_provider) {
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

    for (path, archived) in active_files
        .iter()
        .map(|path| (path, false))
        .chain(archived_files.iter().map(|path| (path, true)))
    {
        match read_session_meta(path) {
            Ok(Some(meta)) => {
                *provider_counts.entry(meta.provider.clone()).or_insert(0) += 1;
                if meta.missing_provider {
                    sessions_missing_provider += 1;
                }
                if meta.provider != current_provider {
                    if !archived {
                        hidden_session_candidates += 1;
                    }
                    if include_archived || !archived {
                        mismatched_session_paths.push(path.clone());
                    }
                }
            }
            Ok(None) => warnings.push(format!("No session_meta found in {}", path.display())),
            Err(error) => warnings.push(format!("Failed to inspect {}: {error:?}", path.display())),
        }
    }

    let index_entries = count_nonempty_lines(&paths.codex_dir.join("session_index.jsonl"));
    let state_stats = inspect_state_database(&paths.codex_dir, &current_provider);
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
        indexed_sessions: index_entries,
        state_database_path: state_stats.path.as_deref().map(display_path),
        state_thread_rows: state_stats.thread_rows,
        state_thread_current_provider: state_stats.current_provider_rows,
        state_thread_other_provider: state_stats.other_provider_rows,
        state_thread_missing_provider: state_stats.missing_provider_rows,
        provider_counts,
        warnings,
    };

    Ok(SessionScan {
        audit,
        mismatched_session_paths,
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

fn read_session_meta(path: &Path) -> Result<Option<SessionMeta>, PluginError> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    for line in reader.lines() {
        let line = line?;
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if value.get("type").and_then(Value::as_str) != Some("session_meta") {
            continue;
        }
        let Some(payload) = value.get("payload").and_then(Value::as_object) else {
            return Ok(None);
        };
        let provider = payload
            .get("model_provider")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        return Ok(Some(SessionMeta {
            provider: provider
                .clone()
                .unwrap_or_else(|| DEFAULT_CODEX_PROVIDER.to_string()),
            missing_provider: provider.is_none(),
        }));
    }
    Ok(None)
}

fn count_nonempty_lines(path: &Path) -> usize {
    let Ok(file) = fs::File::open(path) else {
        return 0;
    };
    BufReader::new(file)
        .lines()
        .map_while(Result::ok)
        .filter(|line| !line.trim().is_empty())
        .count()
}

fn inspect_state_database(codex_dir: &Path, current_provider: &str) -> StateDbStats {
    let Some(path) = latest_state_database(codex_dir) else {
        return StateDbStats {
            path: None,
            thread_rows: 0,
            current_provider_rows: 0,
            other_provider_rows: 0,
            missing_provider_rows: 0,
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
    if !table_has_column(&database, "threads", "model_provider").unwrap_or(false) {
        stats.warnings.push(format!(
            "threads.model_provider column not found in {}",
            path.display()
        ));
        return stats;
    }

    let mut statement = match database.prepare(
        "select coalesce(model_provider, '') as provider, count(*) from threads group by provider",
    ) {
        Ok(statement) => statement,
        Err(error) => {
            stats.warnings.push(format!(
                "failed to inspect threads in {}: {error}",
                path.display()
            ));
            return stats;
        }
    };
    let Ok(rows) = statement.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
    }) else {
        stats.warnings.push(format!(
            "failed to read thread providers in {}",
            path.display()
        ));
        return stats;
    };

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
    stats.threads_to_update = stats.other_provider_rows + stats.missing_provider_rows;
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
    let text = fs::read_to_string(path)?;
    let mut changed = false;
    let mut output = String::new();

    for segment in split_lines_preserve_endings(&text) {
        let line = segment.trim_end_matches(['\r', '\n']);
        let ending = &segment[line.len()..];
        let parsed = serde_json::from_str::<Value>(line);
        match parsed {
            Ok(mut value) if value.get("type").and_then(Value::as_str) == Some("session_meta") => {
                if let Some(payload) = value.get_mut("payload").and_then(Value::as_object_mut) {
                    let current = payload
                        .get("model_provider")
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .unwrap_or(DEFAULT_CODEX_PROVIDER);
                    if current != provider {
                        payload
                            .insert("model_provider".into(), Value::String(provider.to_string()));
                        changed = true;
                        output.push_str(
                            &serde_json::to_string(&value)
                                .map_err(|error| PluginError::Serialization(error.to_string()))?,
                        );
                        output.push_str(ending);
                        continue;
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

fn retag_state_database_threads(path: &Path, provider: &str) -> Result<usize, PluginError> {
    let database = open_database(path, true)?;
    if !table_exists(&database, "threads")?
        || !table_has_column(&database, "threads", "model_provider")?
    {
        return Ok(0);
    }
    database
        .execute(
            "update threads set model_provider = ?1 where model_provider is null or model_provider = '' or model_provider <> ?1",
            [provider],
        )
        .map_err(|error| PluginError::Io(format!("failed to update {}: {error}", path.display())))
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
    let mut statement = database
        .prepare(&format!("pragma table_info({table_name})"))
        .map_err(|error| PluginError::Io(format!("failed to inspect {table_name}: {error}")))?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|error| PluginError::Io(format!("failed to query {table_name}: {error}")))?;
    for row in rows {
        if row.map_err(|error| PluginError::Io(format!("failed to read {table_name}: {error}")))?
            == column_name
        {
            return Ok(true);
        }
    }
    Ok(false)
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
    fn repairs_session_meta_and_state_threads_to_current_provider() {
        let root = std::env::temp_dir().join(format!("switchboard-sessions-{}", timestamp_id()));
        let codex_dir = root.join(".codex");
        let session_dir = codex_dir.join("sessions/2026/07/02");
        fs::create_dir_all(&session_dir).expect("create session dir");
        fs::write(codex_dir.join("config.toml"), "model_provider = \"new\"\n")
            .expect("write config");
        let session_path =
            session_dir.join("rollout-test-11111111-1111-1111-1111-111111111111.jsonl");
        fs::write(
            &session_path,
            "{\"type\":\"session_meta\",\"payload\":{\"id\":\"11111111-1111-1111-1111-111111111111\",\"model_provider\":\"old\"}}\n{\"type\":\"response_item\",\"payload\":{\"type\":\"message\",\"role\":\"user\",\"content\":[{\"type\":\"input_text\",\"text\":\"hello\"}]}}\n",
        )
        .expect("write session");

        let database_path = codex_dir.join("state_5.sqlite");
        let database = Connection::open(&database_path).expect("create database");
        database
            .execute_batch(
                "create table threads(id text primary key, model_provider text);
                 insert into threads(id, model_provider) values('a', 'old');
                 insert into threads(id, model_provider) values('b', null);",
            )
            .expect("seed database");
        drop(database);

        let paths = test_paths(codex_dir.clone());
        let audit = audit_codex_sessions(&paths).expect("audit before");
        assert_eq!(audit.current_provider, "new");
        assert_eq!(audit.hidden_session_candidates, 1);
        assert_eq!(audit.state_thread_other_provider, 1);
        assert_eq!(audit.state_thread_missing_provider, 1);

        let plan = plan_codex_session_provider_repair(&paths, false).expect("plan");
        assert_eq!(plan.session_paths, vec![session_path.clone()]);
        assert_eq!(plan.state_database_path, Some(database_path.clone()));

        let result = apply_codex_session_provider_repair(&paths, plan).expect("repair");
        assert_eq!(result.session_files_changed, 1);
        assert_eq!(result.state_threads_updated, 2);
        assert_eq!(result.audit.hidden_session_candidates, 0);
        assert_eq!(result.audit.state_thread_other_provider, 0);
        assert_eq!(result.audit.state_thread_missing_provider, 0);
        let text = fs::read_to_string(&session_path).expect("read repaired session");
        assert!(text.contains("\"model_provider\":\"new\""));

        let _ = fs::remove_dir_all(root);
    }
}
