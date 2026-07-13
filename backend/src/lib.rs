use hf_plugin_api::{
    declare_plugin, HaloForgePlugin, IpcRegistrar, LogLevel, PluginContext, PluginError,
    PluginMetadata, PLUGIN_ABI_VERSION,
};

mod backup;
mod codex_sessions;
mod commands;
mod fs_util;
mod mcp;
mod paths;
mod provider;
mod status;
mod types;

pub struct SwitchboardPlugin;

impl SwitchboardPlugin {
    pub fn new() -> Self {
        Self
    }
}

impl Default for SwitchboardPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl HaloForgePlugin for SwitchboardPlugin {
    fn metadata(&self) -> PluginMetadata {
        PluginMetadata {
            id: "dev.haloforge.switchboard".into(),
            name: "Switchboard".into(),
            version: "0.1.16".into(),
            description: "Fast local configuration switching for Claude Code, Codex, and MCP."
                .into(),
            author: "HaloForge Team".into(),
            abi_version: PLUGIN_ABI_VERSION,
        }
    }

    fn on_load(
        &mut self,
        ctx: &dyn PluginContext,
        ipc: &mut dyn IpcRegistrar,
    ) -> Result<(), PluginError> {
        ipc.register("switchboard_status", Box::new(commands::switchboard_status))?;
        ipc.register(
            "switchboard_apply_provider",
            Box::new(commands::switchboard_apply_provider),
        )?;
        ipc.register(
            "switchboard_list_backups",
            Box::new(commands::switchboard_list_backups),
        )?;
        ipc.register(
            "switchboard_restore_backup",
            Box::new(commands::switchboard_restore_backup),
        )?;
        ipc.register(
            "switchboard_cleanup_codex",
            Box::new(commands::switchboard_cleanup_codex),
        )?;
        ipc.register(
            "switchboard_codex_session_audit",
            Box::new(commands::switchboard_codex_session_audit),
        )?;
        ipc.register(
            "switchboard_repair_codex_sessions",
            Box::new(commands::switchboard_repair_codex_sessions),
        )?;
        ipc.register(
            "switchboard_install_mcp",
            Box::new(commands::switchboard_install_mcp),
        )?;
        ipc.register(
            "switchboard_discover_models",
            Box::new(commands::switchboard_discover_models),
        )?;

        ctx.log(LogLevel::Info, "Switchboard plugin loaded");
        Ok(())
    }

    fn on_unload(&mut self) -> Result<(), PluginError> {
        Ok(())
    }
}

declare_plugin!(SwitchboardPlugin, SwitchboardPlugin::new);
