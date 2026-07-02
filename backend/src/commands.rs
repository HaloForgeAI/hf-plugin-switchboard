use crate::fs_util::{display_path, pathbufs_to_strings};
use crate::paths::SwitchboardPaths;
use crate::types::{
    ApplyProviderArgs, ApplyProviderResult, CleanupCodexArgs, CleanupCodexResult,
    CodexSessionRepairArgs, CodexSessionRepairResult, DiscoverModelsArgs, InstallMcpArgs,
    InstallMcpResult, RestoreBackupArgs, RestoreBackupResult,
};
use crate::{backup, codex_sessions, mcp, provider, status};
use hf_plugin_api::{PluginContext, PluginError};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;

pub fn switchboard_status(_args: Value, ctx: &dyn PluginContext) -> Result<Value, PluginError> {
    to_value(status::read_status(ctx)?)
}

pub fn switchboard_apply_provider(
    args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: ApplyProviderArgs = parse_args(args)?;
    provider::validate_provider_args(&args)?;

    let paths = SwitchboardPaths::resolve()?;
    let backup = backup::create_backup(ctx, provider::provider_backup_paths(&paths, &args))?;
    let touched = provider::apply_provider(&paths, &args)?;

    to_value(ApplyProviderResult {
        backup,
        changed_paths: pathbufs_to_strings(&touched),
        target: args.target,
    })
}

pub fn switchboard_list_backups(
    _args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    to_value(backup::list_backup_infos(ctx)?)
}

pub fn switchboard_restore_backup(
    args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: RestoreBackupArgs = parse_args(args)?;
    let backup = backup::read_backup_info(ctx, &args.backup_id)?;
    let backup_dir = backup::backup_dir(ctx).join(&backup.id);
    let original_paths = backup
        .files
        .iter()
        .map(|file| PathBuf::from(&file.original_path))
        .collect::<Vec<_>>();
    let safety_backup = backup::create_backup(ctx, original_paths)?;
    let mut restored = Vec::new();
    let mut seen = BTreeSet::new();

    for file in &backup.files {
        let original = PathBuf::from(&file.original_path);
        let original_display = display_path(&original);
        if !seen.insert(original_display.clone()) {
            continue;
        }
        if file.existed {
            let rel = file
                .backup_file
                .as_deref()
                .ok_or_else(|| PluginError::Custom("backup manifest missing backup file".into()))?;
            let source = safe_backup_source(&backup_dir, rel)?;
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
        restored.push(original_display);
    }

    to_value(RestoreBackupResult {
        restored_paths: restored,
        safety_backup,
    })
}

pub fn switchboard_cleanup_codex(
    args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: CleanupCodexArgs = parse_args(args)?;
    let paths = SwitchboardPaths::resolve()?;
    let backup = backup::create_backup(ctx, provider::codex_cleanup_backup_paths(&paths))?;
    let touched = provider::cleanup_codex_custom_api(&paths, &args)?;

    to_value(CleanupCodexResult {
        backup,
        changed_paths: pathbufs_to_strings(&touched),
    })
}

pub fn switchboard_codex_session_audit(
    _args: Value,
    _ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let paths = SwitchboardPaths::resolve()?;
    to_value(codex_sessions::audit_codex_sessions(&paths)?)
}

pub fn switchboard_repair_codex_sessions(
    args: Value,
    ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: CodexSessionRepairArgs = parse_args(args)?;
    let paths = SwitchboardPaths::resolve()?;
    let plan = codex_sessions::plan_codex_session_provider_repair(
        &paths,
        args.include_archived.unwrap_or(false),
    )?;
    let backup = backup::create_backup(ctx, plan.backup_paths.clone())?;
    let target_provider = plan.target_provider.clone();
    let result = codex_sessions::apply_codex_session_provider_repair(&paths, plan)?;

    to_value(CodexSessionRepairResult {
        backup,
        changed_paths: codex_sessions::changed_paths_for_result(&result.changed_paths),
        session_files_changed: result.session_files_changed,
        index_entries_written: result.index_entries_written,
        state_threads_updated: result.state_threads_updated,
        target_provider,
        audit: result.audit,
        warnings: result.warnings,
    })
}

pub fn switchboard_install_mcp(args: Value, ctx: &dyn PluginContext) -> Result<Value, PluginError> {
    let args: InstallMcpArgs = parse_args(args)?;
    mcp::validate_mcp_args(&args)?;

    let paths = SwitchboardPaths::resolve()?;
    let _backup = backup::create_backup(ctx, mcp::mcp_backup_paths(&paths, &args))?;
    let touched = mcp::install_mcp(&paths, &args)?;

    to_value(InstallMcpResult {
        changed_paths: pathbufs_to_strings(&touched),
    })
}

pub fn switchboard_discover_models(
    args: Value,
    _ctx: &dyn PluginContext,
) -> Result<Value, PluginError> {
    let args: DiscoverModelsArgs = parse_args(args)?;
    provider::validate_models_args(&args)?;
    to_value(provider::discover_models(&args)?)
}

fn parse_args<T: for<'de> Deserialize<'de>>(args: Value) -> Result<T, PluginError> {
    serde_json::from_value(args).map_err(|error| PluginError::Serialization(error.to_string()))
}

fn to_value<T: Serialize>(value: T) -> Result<Value, PluginError> {
    serde_json::to_value(value).map_err(|error| PluginError::Serialization(error.to_string()))
}

fn safe_backup_source(backup_dir: &std::path::Path, rel: &str) -> Result<PathBuf, PluginError> {
    let rel_path = std::path::Path::new(rel);
    if rel_path.is_absolute()
        || rel_path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err(PluginError::Custom(
            "backup manifest contains an unsafe file path".into(),
        ));
    }
    Ok(backup_dir.join(rel_path))
}
