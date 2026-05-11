use hf_plugin_api::PluginError;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct SwitchboardPaths {
    pub home: PathBuf,
    pub claude_settings_path: PathBuf,
    pub claude_config_path: PathBuf,
    pub claude_mcp_path: PathBuf,
    pub codex_auth_path: PathBuf,
    pub codex_config_path: PathBuf,
}

impl SwitchboardPaths {
    pub fn resolve() -> Result<Self, PluginError> {
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
            home: home.clone(),
            claude_settings_path,
            claude_config_path: claude_dir.join("config.json"),
            claude_mcp_path: home.join(".claude.json"),
            codex_auth_path: codex_dir.join("auth.json"),
            codex_config_path: codex_dir.join("config.toml"),
        })
    }
}
