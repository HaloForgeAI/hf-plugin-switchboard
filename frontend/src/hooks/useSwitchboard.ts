import { invokePlugin } from "@haloforge/plugin-sdk";
import { useCallback, useEffect, useState } from "react";
import { useSwitchboardT } from "../i18n";
import type {
  BackupInfo,
  CleanupCodexForm,
  CodexLogFixStatus,
  McpAppSelection,
  ProviderForm,
  SwitchboardStatus,
} from "../types";

export function useSwitchboard() {
  const t = useSwitchboardT();
  const [status, setStatus] = useState<SwitchboardStatus | null>(null);
  const [codexLogFixStatus, setCodexLogFixStatus] = useState<CodexLogFixStatus | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  const refresh = useCallback(async () => {
    const next = await invokePlugin<SwitchboardStatus>("switchboard_status", {});
    setStatus(next);
  }, []);

  useEffect(() => {
    void refresh().catch((error) => setMessage(formatError(error)));
  }, [refresh]);

  const applyProvider = useCallback(
    async (form: ProviderForm) => {
      setBusy("provider");
      setMessage(null);
      try {
        const result = await invokePlugin<{ changedPaths: string[]; backup: BackupInfo }>(
          "switchboard_apply_provider",
          { ...form },
        );
        setMessage(t("switchboard.message.providerApplied", {
          count: result.changedPaths.length,
          backupId: result.backup.id,
        }));
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh, t],
  );

  const installMcp = useCallback(
    async (id: string, apps: McpAppSelection, specText: string) => {
      setBusy("mcp");
      setMessage(null);
      try {
        const spec = JSON.parse(specText);
        const selectedApps = Object.entries(apps)
          .filter(([, enabled]) => enabled)
          .map(([app]) => app);
        const result = await invokePlugin<{ changedPaths: string[] }>("switchboard_install_mcp", {
          id,
          apps: selectedApps,
          spec,
        });
        setMessage(t("switchboard.message.mcpInstalled", { count: result.changedPaths.length }));
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh, t],
  );

  const cleanupCodex = useCallback(
    async (form: CleanupCodexForm) => {
      setBusy("cleanup-codex");
      setMessage(null);
      try {
        const result = await invokePlugin<{ changedPaths: string[]; backup: BackupInfo }>(
          "switchboard_cleanup_codex",
          { ...form },
        );
        setMessage(t("switchboard.message.codexCleaned", {
          count: result.changedPaths.length,
          backupId: result.backup.id,
        }));
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh, t],
  );

  const checkCodexLogFix = useCallback(
    async () => {
      setBusy("codex-log-fix-check");
      setMessage(null);
      try {
        const result = await invokePlugin<CodexLogFixStatus>("switchboard_codex_log_fix_status", {});
        setCodexLogFixStatus(result);
        setMessage(t("switchboard.message.codexLogFixChecked"));
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [t],
  );

  const applyCodexLogFix = useCallback(
    async () => {
      setBusy("codex-log-fix-apply");
      setMessage(null);
      try {
        const result = await invokePlugin<CodexLogFixStatus>("switchboard_apply_codex_log_fix", {});
        setCodexLogFixStatus(result);
        setMessage(t("switchboard.message.codexLogFixApplied"));
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [t],
  );

  const discoverModels = useCallback(
    async (form: Pick<ProviderForm, "baseUrl" | "apiKey" | "modelsPath">) => {
      setBusy("models");
      setMessage(null);
      try {
        const result = await invokePlugin<{ models: string[] }>("switchboard_discover_models", {
          baseUrl: form.baseUrl,
          apiKey: form.apiKey,
          modelsPath: form.modelsPath,
        });
        if (result.models.length === 0) {
          setMessage(t("switchboard.message.modelsEmpty"));
        } else {
          setMessage(t("switchboard.message.modelsLoaded", { count: result.models.length }));
        }
        return result.models;
      } catch (error) {
        setMessage(formatError(error));
        return [];
      } finally {
        setBusy(null);
      }
    },
    [t],
  );

  const restoreBackup = useCallback(
    async (backupId: string) => {
      setBusy(`restore:${backupId}`);
      setMessage(null);
      try {
        const result = await invokePlugin<{ restoredPaths: string[] }>("switchboard_restore_backup", {
          backupId,
        });
        setMessage(t("switchboard.message.backupRestored", { count: result.restoredPaths.length }));
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh, t],
  );

  return {
    status,
    codexLogFixStatus,
    busy,
    message,
    setMessage,
    refresh,
    applyProvider,
    installMcp,
    cleanupCodex,
    checkCodexLogFix,
    applyCodexLogFix,
    discoverModels,
    restoreBackup,
  };
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}
