import { invokePlugin } from "@haloforge/plugin-sdk";
import { useCallback, useEffect, useState } from "react";
import { useSwitchboardT } from "../i18n";
import type { BackupInfo, McpAppSelection, ProviderForm, SwitchboardStatus } from "../types";

export function useSwitchboard() {
  const t = useSwitchboardT();
  const [status, setStatus] = useState<SwitchboardStatus | null>(null);
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
    busy,
    message,
    setMessage,
    refresh,
    applyProvider,
    installMcp,
    discoverModels,
    restoreBackup,
  };
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}
