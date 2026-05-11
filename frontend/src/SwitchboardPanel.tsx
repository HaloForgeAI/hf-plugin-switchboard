import { usePluginSettings } from "@haloforge/plugin-sdk";
import { RefreshCcw } from "lucide-react";
import { useEffect, useState } from "react";
import { BackupPanel } from "./components/BackupPanel";
import { McpPanel } from "./components/McpPanel";
import { ProviderPanel } from "./components/ProviderPanel";
import { TargetCard } from "./components/TargetCard";
import { DEFAULT_MCP_SPEC, defaultProviderForm } from "./defaults";
import { useSwitchboard } from "./hooks/useSwitchboard";
import { formatError, parseCcSwitchMcpUrl, parseCcSwitchProviderUrl } from "./deepLinkImport";
import type { McpAppSelection, PluginSettings } from "./types";

export function SwitchboardPanel() {
  const { settings } = usePluginSettings<PluginSettings>();
  const [form, setForm] = useState(() => defaultProviderForm({}));
  const [ccswitchUrl, setCcswitchUrl] = useState("");
  const [mcpId, setMcpId] = useState("context7");
  const [mcpImportUrl, setMcpImportUrl] = useState("");
  const [mcpApps, setMcpApps] = useState<McpAppSelection>({
    claude: true,
    codex: true,
  });
  const [mcpSpec, setMcpSpec] = useState(DEFAULT_MCP_SPEC);
  const {
    status,
    busy,
    message,
    setMessage,
    refresh,
    applyProvider,
    installMcp,
    restoreBackup,
  } = useSwitchboard();

  useEffect(() => {
    setForm((current) => ({
      ...current,
      target: current.target || settings.defaultTarget || "both",
      providerId: current.providerId || settings.stableCodexProviderId || "switchboard",
    }));
  }, [settings.defaultTarget, settings.stableCodexProviderId]);

  const importCcSwitchUrl = () => {
    try {
      const patch = parseCcSwitchProviderUrl(ccswitchUrl, form.providerId);
      setForm((current) => ({ ...current, ...patch }));
      setMessage("Imported provider fields.");
    } catch (error) {
      setMessage(formatError(error));
    }
  };

  const importMcpUrl = () => {
    try {
      const patch = parseCcSwitchMcpUrl(mcpImportUrl);
      setMcpId(patch.id);
      setMcpApps(patch.apps);
      setMcpSpec(patch.specText);
      setMessage("Imported MCP fields.");
    } catch (error) {
      setMessage(formatError(error));
    }
  };

  const backupCount = status?.backups.length ?? 0;
  const configuredCount = status?.targets.filter((target) => target.configured).length ?? 0;

  return (
    <main className="sb-shell">
      <header className="sb-topbar">
        <div>
          <h1>Switchboard</h1>
          <div className="sb-meta">
            <span>{status?.os ?? "local"}</span>
            <span>{configuredCount} targets</span>
            <span>{backupCount} backups</span>
          </div>
        </div>
        <button className="sb-icon-button" type="button" onClick={() => void refresh()} title="Refresh">
          <RefreshCcw size={17} />
        </button>
      </header>

      {message && <div className="sb-banner">{message}</div>}

      <section className="sb-status-grid">
        {status?.targets.map((target) => (
          <TargetCard key={target.id} target={target} />
        ))}
      </section>

      <section className="sb-grid">
        <ProviderPanel
          form={form}
          setForm={setForm}
          ccswitchUrl={ccswitchUrl}
          setCcswitchUrl={setCcswitchUrl}
          busy={busy}
          onImport={importCcSwitchUrl}
          onApply={() => void applyProvider(form)}
        />
        <McpPanel
          mcpId={mcpId}
          setMcpId={setMcpId}
          mcpImportUrl={mcpImportUrl}
          setMcpImportUrl={setMcpImportUrl}
          mcpApps={mcpApps}
          setMcpApps={setMcpApps}
          mcpSpec={mcpSpec}
          setMcpSpec={setMcpSpec}
          busy={busy}
          onImport={importMcpUrl}
          onInstall={() => void installMcp(mcpId, mcpApps, mcpSpec)}
        />
        <BackupPanel backups={status?.backups ?? []} busy={busy} onRestore={(id) => void restoreBackup(id)} />
      </section>
    </main>
  );
}
