import { invokePlugin } from "@haloforge/plugin-sdk";
import { useCallback, useEffect, useState } from "react";
import { formatError } from "../deepLinkImport";
import type { BackupInfo, McpAppSelection, ProviderForm, SwitchboardStatus } from "../types";

export function useSwitchboard() {
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
        setMessage(`Applied ${result.changedPaths.length} file update(s). Backup ${result.backup.id}.`);
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh],
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
        setMessage(`Installed MCP into ${result.changedPaths.length} config file(s).`);
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh],
  );

  const restoreBackup = useCallback(
    async (backupId: string) => {
      setBusy(`restore:${backupId}`);
      setMessage(null);
      try {
        const result = await invokePlugin<{ restoredPaths: string[] }>("switchboard_restore_backup", {
          backupId,
        });
        setMessage(`Restored ${result.restoredPaths.length} file(s).`);
        await refresh();
      } catch (error) {
        setMessage(formatError(error));
      } finally {
        setBusy(null);
      }
    },
    [refresh],
  );

  return {
    status,
    busy,
    message,
    setMessage,
    refresh,
    applyProvider,
    installMcp,
    restoreBackup,
  };
}
