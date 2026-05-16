import { usePluginSettings } from "@haloforge/plugin-sdk";
import { Bot, Braces, LayoutDashboard, RefreshCcw, Shield, TerminalSquare } from "lucide-react";
import { useEffect, useState } from "react";
import { BackupPanel } from "./components/BackupPanel";
import { McpPanel } from "./components/McpPanel";
import { ProviderPanel } from "./components/ProviderPanel";
import { TargetCard } from "./components/TargetCard";
import { DEFAULT_CODEX_PROVIDER_ID, DEFAULT_MCP_SPEC, defaultProviderForm } from "./defaults";
import { useSwitchboardT } from "./i18n";
import { useSwitchboard } from "./hooks/useSwitchboard";
import type { McpAppSelection, PluginSettings, ProviderForm, TargetStatus } from "./types";

type SwitchboardTab = "overview" | "claude" | "codex" | "mcp" | "backups";

function buildProviderForm(settings: PluginSettings, target: ProviderForm["target"]): ProviderForm {
  return {
    ...defaultProviderForm(settings),
    target,
  };
}

export function SwitchboardPanel() {
  const t = useSwitchboardT();
  const { settings } = usePluginSettings<PluginSettings>();
  const [activeTab, setActiveTab] = useState<SwitchboardTab>("overview");
  const [claudeForm, setClaudeForm] = useState(() => buildProviderForm({}, "claude"));
  const [codexForm, setCodexForm] = useState(() => buildProviderForm({}, "codex"));
  const [mcpId, setMcpId] = useState("context7");
  const [mcpApps, setMcpApps] = useState<McpAppSelection>({
    claude: true,
    codex: true,
  });
  const [mcpSpec, setMcpSpec] = useState(DEFAULT_MCP_SPEC);
  const {
    status,
    busy,
    message,
    refresh,
    applyProvider,
    installMcp,
    discoverModels,
    restoreBackup,
  } = useSwitchboard();

  useEffect(() => {
    setClaudeForm((current) => ({
      ...current,
      target: "claude",
      providerId: current.providerId || settings.stableCodexProviderId || DEFAULT_CODEX_PROVIDER_ID,
      name: settings.providerName?.trim() || current.name,
      model: current.model || settings.defaultModel?.trim() || "",
      modelsPath: current.modelsPath || settings.modelsPath?.trim() || "/models",
    }));
    setCodexForm((current) => ({
      ...current,
      target: "codex",
      providerId: current.providerId || settings.stableCodexProviderId || DEFAULT_CODEX_PROVIDER_ID,
      name: settings.providerName?.trim() || current.name,
      model: current.model || settings.defaultModel?.trim() || "",
      modelsPath: current.modelsPath || settings.modelsPath?.trim() || "/models",
    }));
  }, [settings.stableCodexProviderId, settings.providerName, settings.defaultModel, settings.modelsPath]);

  const configuredCount = status?.targets.filter((target) => target.configured).length ?? 0;
  const backupCount = status?.backups.length ?? 0;
  const claudeStatus = status?.targets.find((target) => target.id === "claude");
  const codexStatus = status?.targets.find((target) => target.id === "codex");

  const tabs: Array<{
    id: SwitchboardTab;
    label: string;
    icon: typeof LayoutDashboard;
  }> = [
    { id: "overview", label: t("switchboard.tab.overview"), icon: LayoutDashboard },
    { id: "claude", label: t("switchboard.tab.claude"), icon: Bot },
    { id: "codex", label: t("switchboard.tab.codex"), icon: Braces },
    { id: "mcp", label: t("switchboard.tab.mcp"), icon: TerminalSquare },
    { id: "backups", label: t("switchboard.tab.backups"), icon: Shield },
  ];

  return (
    <main className="sb-shell">
      <header className="sb-topbar">
        <div>
          <h1>{t("switchboard.title")}</h1>
          <p className="sb-subtitle">{t("switchboard.subtitle")}</p>
          <div className="sb-meta">
            <span>{t("switchboard.meta.os", { value: status?.os ?? "local" })}</span>
            <span>{t("switchboard.meta.targets", { count: configuredCount })}</span>
            <span>{t("switchboard.meta.backups", { count: backupCount })}</span>
          </div>
        </div>
        <button className="sb-icon-button" type="button" onClick={() => void refresh()} title={t("switchboard.refresh")}>
          <RefreshCcw size={17} />
        </button>
      </header>

      <nav className="sb-tabbar" aria-label="Provider router sections">
        {tabs.map((tab) => {
          const Icon = tab.icon;
          return (
            <button
              key={tab.id}
              type="button"
              className={activeTab === tab.id ? "sb-tab-button sb-tab-button-active" : "sb-tab-button"}
              onClick={() => setActiveTab(tab.id)}
            >
              <Icon size={15} />
              <span>{tab.label}</span>
            </button>
          );
        })}
      </nav>

      {message && <div className="sb-banner">{message}</div>}

      {activeTab === "overview" && (
        <section className="sb-pane">
          <div className="sb-section-heading">
            <div>
              <h2>{t("switchboard.overview.title")}</h2>
              <p>{t("switchboard.overview.subtitle")}</p>
            </div>
          </div>

          <div className="sb-status-grid">
            {status?.targets.map((target) => (
              <TargetCard key={target.id} target={target} t={t} />
            ))}
            {!status?.targets.length && <div className="sb-empty">{t("switchboard.overview.empty")}</div>}
          </div>

          <div className="sb-overview-grid">
            <section className="sb-panel">
              <div className="sb-section-heading">
                <div>
                  <h2>{t("switchboard.overview.backupTitle")}</h2>
                  <p>{t("switchboard.overview.backupBody")}</p>
                </div>
              </div>
              <div className="sb-summary-stats">
                <div className="sb-stat-card">
                  <span>{t("switchboard.meta.backups", { count: backupCount })}</span>
                  <strong>{backupCount}</strong>
                </div>
                <div className="sb-stat-card">
                  <span>{t("switchboard.overview.dataDir")}</span>
                  <code>{status?.dataDir ?? "-"}</code>
                </div>
              </div>
            </section>
          </div>
        </section>
      )}

      {activeTab === "claude" && (
        <section className="sb-pane">
          <ProviderPanel
            target="claude"
            status={claudeStatus}
            form={claudeForm}
            setForm={setClaudeForm}
            busy={busy}
            onApply={() => void applyProvider(claudeForm)}
            onDiscoverModels={() => discoverModels(claudeForm)}
            t={t}
          />
        </section>
      )}

      {activeTab === "codex" && (
        <section className="sb-pane">
          <ProviderPanel
            target="codex"
            status={codexStatus}
            form={codexForm}
            setForm={setCodexForm}
            busy={busy}
            onApply={() => void applyProvider(codexForm)}
            onDiscoverModels={() => discoverModels(codexForm)}
            t={t}
          />
        </section>
      )}

      {activeTab === "mcp" && (
        <section className="sb-pane">
          <McpPanel
            mcpId={mcpId}
            setMcpId={setMcpId}
            mcpApps={mcpApps}
            setMcpApps={setMcpApps}
            mcpSpec={mcpSpec}
            setMcpSpec={setMcpSpec}
            busy={busy}
            onInstall={() => void installMcp(mcpId, mcpApps, mcpSpec)}
            t={t}
          />
        </section>
      )}

      {activeTab === "backups" && (
        <section className="sb-pane">
          <BackupPanel backups={status?.backups ?? []} busy={busy} onRestore={(id) => void restoreBackup(id)} t={t} />
        </section>
      )}
    </main>
  );
}
