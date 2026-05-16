use hf_plugin_api::PluginError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn read_json(path: &Path) -> Result<Value, PluginError> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        PluginError::Serialization(format!("failed to parse {}: {error}", path.display()))
    })
}

pub fn read_json_object_or_empty(path: &Path) -> Result<Value, PluginError> {
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

pub fn read_json_typed<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, PluginError> {
    let text = fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|error| {
        PluginError::Serialization(format!("failed to parse {}: {error}", path.display()))
    })
}

pub fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<(), PluginError> {
    let mut content = serde_json::to_vec_pretty(value)
        .map_err(|error| PluginError::Serialization(error.to_string()))?;
    content.push(b'\n');
    atomic_write(path, &content)
}

pub fn atomic_write(path: &Path, content: &[u8]) -> Result<(), PluginError> {
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

pub fn read_optional_bytes(path: &Path) -> Result<Option<Vec<u8>>, PluginError> {
    if path.exists() {
        Ok(Some(fs::read(path)?))
    } else {
        Ok(None)
    }
}

pub fn restore_optional_bytes(path: &Path, bytes: Option<Vec<u8>>) -> Result<(), PluginError> {
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

pub fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub fn pathbufs_to_strings(paths: &[PathBuf]) -> Vec<String> {
    paths.iter().map(|path| display_path(path)).collect()
}

pub fn mask_secret(secret: &str) -> String {
    let trimmed = secret.trim();
    if trimmed.len() <= 8 {
        return "****".to_string();
    }
    format!("{}...{}", &trimmed[..4], &trimmed[trimmed.len() - 4..])
}

pub fn timestamp_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    nanos.to_string()
}

pub fn timestamp_display() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    secs.to_string()
}
