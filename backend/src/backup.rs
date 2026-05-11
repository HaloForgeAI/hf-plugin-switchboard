use crate::fs_util::{display_path, read_json_typed, timestamp_id, write_json_pretty};
use crate::types::{BackupFile, BackupInfo};
use hf_plugin_api::{PluginContext, PluginError};
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

pub fn create_backup(
    ctx: &dyn PluginContext,
    paths: impl IntoIterator<Item = PathBuf>,
) -> Result<BackupInfo, PluginError> {
    let id = timestamp_id();
    let dir = backup_dir(ctx).join(&id);
    let files_dir = dir.join("files");
    fs::create_dir_all(&files_dir)?;

    let mut files = Vec::new();
    let mut seen = BTreeSet::new();
    for path in paths {
        let path_text = display_path(&path);
        if !seen.insert(path_text.clone()) {
            continue;
        }
        let index = files.len();
        if path.exists() {
            let backup_file = format!("files/{index}");
            fs::copy(&path, dir.join(&backup_file)).map_err(|error| {
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

pub fn list_backup_infos(ctx: &dyn PluginContext) -> Result<Vec<BackupInfo>, PluginError> {
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

pub fn read_backup_info(
    ctx: &dyn PluginContext,
    backup_id: &str,
) -> Result<BackupInfo, PluginError> {
    let safe = sanitize_backup_id(backup_id)?;
    read_json_typed(&backup_dir(ctx).join(safe).join("manifest.json"))
}

pub fn backup_dir(ctx: &dyn PluginContext) -> PathBuf {
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
