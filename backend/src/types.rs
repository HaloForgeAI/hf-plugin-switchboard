use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const CLAUDE_TARGET: &str = "claude";
pub const CODEX_TARGET: &str = "codex";
pub const BOTH_TARGET: &str = "both";
pub const DEFAULT_CODEX_PROVIDER_ID: &str = "openai";
pub const PREVIOUS_DEFAULT_CODEX_PROVIDER_ID: &str = "haloforge_gateway";
pub const LEGACY_CODEX_PROVIDER_ID: &str = "switchboard";

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PathStatus {
    pub label: String,
    pub path: String,
    pub exists: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStatus {
    pub id: String,
    pub label: String,
    pub configured: bool,
    pub summary: Option<String>,
    pub details: Vec<TargetDetail>,
    pub paths: Vec<PathStatus>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetDetail {
    pub label: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwitchboardStatus {
    pub os: String,
    pub home_dir: Option<String>,
    pub data_dir: String,
    pub targets: Vec<TargetStatus>,
    pub backups: Vec<BackupInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub id: String,
    pub created_at: String,
    pub path: String,
    pub files: Vec<BackupFile>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct BackupFile {
    pub original_path: String,
    pub backup_file: Option<String>,
    pub existed: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub byte_count: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preview: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProviderArgs {
    pub target: String,
    pub name: String,
    pub base_url: String,
    pub api_key: String,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub reasoning_effort: Option<String>,
    pub haiku_model: Option<String>,
    pub sonnet_model: Option<String>,
    pub opus_model: Option<String>,
    pub set_claude_primary_api_key: Option<bool>,
    pub skip_claude_onboarding: Option<bool>,
    #[serde(alias = "enableCodexChromePlugin")]
    pub enable_codex_builtin_plugins: Option<bool>,
    pub preserve_codex_chatgpt_auth: Option<bool>,
    pub codex_auth_mode: Option<String>,
    pub codex_env_key: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplyProviderResult {
    pub backup: BackupInfo,
    pub changed_paths: Vec<String>,
    pub target: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupArgs {
    pub backup_id: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreBackupResult {
    pub restored_paths: Vec<String>,
    pub safety_backup: BackupInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCodexArgs {
    pub provider_id: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCodexResult {
    pub backup: BackupInfo,
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionProviderCount {
    pub provider: String,
    pub count: usize,
    pub current: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionAudit {
    pub codex_home: String,
    pub current_provider: String,
    pub session_files: usize,
    pub archived_session_files: usize,
    pub sessions_missing_provider: usize,
    pub hidden_session_candidates: usize,
    pub indexed_sessions: usize,
    pub index_duplicate_entries: usize,
    pub index_missing_sessions: usize,
    pub state_database_path: Option<String>,
    pub state_thread_rows: usize,
    pub state_thread_current_provider: usize,
    pub state_thread_other_provider: usize,
    pub state_thread_missing_provider: usize,
    pub state_thread_missing_sessions: usize,
    pub provider_counts: Vec<CodexSessionProviderCount>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionRepairArgs {
    pub include_archived: Option<bool>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CodexSessionRepairResult {
    pub backup: BackupInfo,
    pub changed_paths: Vec<String>,
    pub session_files_changed: usize,
    pub index_entries_written: usize,
    pub state_threads_updated: usize,
    pub target_provider: String,
    pub audit: CodexSessionAudit,
    pub warnings: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallMcpArgs {
    pub id: String,
    pub apps: Vec<String>,
    pub spec: Value,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallMcpResult {
    pub changed_paths: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverModelsArgs {
    pub base_url: String,
    pub api_key: String,
    pub models_path: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoverModelsResult {
    pub models: Vec<String>,
}
