use crate::fs_util::{display_path, pathbufs_to_strings};
use crate::paths::SwitchboardPaths;
use crate::types::CodexLogFixResult;
use hf_plugin_api::PluginError;
use rusqlite::{Connection, OpenFlags};
use std::path::{Path, PathBuf};
use std::time::Duration;

const CODEX_LOG_DATABASE_NAME: &str = "logs_2.sqlite";
const CODEX_LOG_TRIGGER_NAME: &str = "block_log_inserts";
const CODEX_LOG_TRIGGER_SQL: &str = r#"
CREATE TRIGGER IF NOT EXISTS block_log_inserts
BEFORE INSERT ON logs
BEGIN
  SELECT RAISE(IGNORE);
END;
"#;

pub fn codex_log_fix_status(paths: &SwitchboardPaths) -> Result<CodexLogFixResult, PluginError> {
    let candidates = candidate_database_paths(paths);
    let Some(database_path) = find_existing_database(&candidates) else {
        return Ok(result(
            candidates,
            None,
            "not_found",
            "No Codex logs_2.sqlite database was found.",
        ));
    };

    let database = open_database(&database_path, false)?;
    if !sqlite_object_exists(&database, "table", "logs")? {
        return Ok(result(
            candidates,
            Some(database_path),
            "unsupported",
            "The Codex log database does not contain a logs table.",
        ));
    }

    if sqlite_object_exists(&database, "trigger", CODEX_LOG_TRIGGER_NAME)? {
        Ok(result(
            candidates,
            Some(database_path),
            "applied",
            "The SQLite log insert blocker is already installed.",
        ))
    } else {
        Ok(result(
            candidates,
            Some(database_path),
            "ready",
            "The Codex log database is ready for the SQLite log insert blocker.",
        ))
    }
}

pub fn apply_codex_log_fix(paths: &SwitchboardPaths) -> Result<CodexLogFixResult, PluginError> {
    let candidates = candidate_database_paths(paths);
    let Some(database_path) = find_existing_database(&candidates) else {
        return Ok(result(
            candidates,
            None,
            "not_found",
            "No Codex logs_2.sqlite database was found.",
        ));
    };

    install_log_insert_blocker(&database_path)?;
    Ok(result(
        candidates,
        Some(database_path),
        "applied",
        "Installed the SQLite log insert blocker.",
    ))
}

fn candidate_database_paths(paths: &SwitchboardPaths) -> Vec<PathBuf> {
    let codex_home = std::env::var_os("CODEX_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| paths.home.join(".codex"));
    vec![
        codex_home.join(CODEX_LOG_DATABASE_NAME),
        codex_home.join("sqlite").join(CODEX_LOG_DATABASE_NAME),
    ]
}

fn find_existing_database(candidates: &[PathBuf]) -> Option<PathBuf> {
    candidates.iter().find(|path| path.exists()).cloned()
}

fn install_log_insert_blocker(database_path: &Path) -> Result<(), PluginError> {
    let database = open_database(database_path, true)?;
    if !sqlite_object_exists(&database, "table", "logs")? {
        return Err(PluginError::Custom(format!(
            "{} does not contain a logs table",
            database_path.display()
        )));
    }
    database
        .execute_batch(CODEX_LOG_TRIGGER_SQL)
        .map_err(|error| {
            PluginError::Io(format!(
                "failed to install {} in {}: {error}",
                CODEX_LOG_TRIGGER_NAME,
                database_path.display()
            ))
        })
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

fn sqlite_object_exists(
    database: &Connection,
    object_type: &str,
    name: &str,
) -> Result<bool, PluginError> {
    let mut statement = database
        .prepare("select 1 from sqlite_master where type = ?1 and name = ?2 limit 1")
        .map_err(|error| PluginError::Io(format!("failed to inspect SQLite schema: {error}")))?;
    let mut rows = statement
        .query([object_type, name])
        .map_err(|error| PluginError::Io(format!("failed to query SQLite schema: {error}")))?;
    rows.next()
        .map(|row| row.is_some())
        .map_err(|error| PluginError::Io(format!("failed to read SQLite schema row: {error}")))
}

fn result(
    candidates: Vec<PathBuf>,
    database_path: Option<PathBuf>,
    status: &str,
    message: &str,
) -> CodexLogFixResult {
    CodexLogFixResult {
        candidate_paths: pathbufs_to_strings(&candidates),
        database_path: database_path.as_deref().map(display_path),
        status: status.to_string(),
        trigger_name: CODEX_LOG_TRIGGER_NAME.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs_util::timestamp_id;

    fn temp_database_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("switchboard-{name}-{}.sqlite", timestamp_id()))
    }

    #[test]
    fn installs_sqlite_log_insert_blocker() {
        let path = temp_database_path("codex-log-fix");
        let database = Connection::open(&path).expect("create database");
        database
            .execute_batch("create table logs(id integer primary key, body text);")
            .expect("create logs table");
        drop(database);

        install_log_insert_blocker(&path).expect("install blocker");

        let database = Connection::open(&path).expect("open database");
        assert!(sqlite_object_exists(&database, "trigger", CODEX_LOG_TRIGGER_NAME).unwrap());
        database
            .execute("insert into logs(body) values('ignored')", [])
            .expect("insert ignored");
        let count: i64 = database
            .query_row("select count(*) from logs", [], |row| row.get(0))
            .expect("count rows");
        assert_eq!(count, 0);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn refuses_database_without_logs_table() {
        let path = temp_database_path("codex-log-fix-no-table");
        let database = Connection::open(&path).expect("create database");
        database
            .execute_batch("create table other(id integer primary key);")
            .expect("create other table");
        drop(database);

        let error = install_log_insert_blocker(&path).expect_err("missing logs table");
        assert!(format!("{error:?}").contains("logs table"));
        let _ = std::fs::remove_file(path);
    }
}
