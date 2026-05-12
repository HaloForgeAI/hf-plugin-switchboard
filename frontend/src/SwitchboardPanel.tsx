import { usePluginSettings } from "@haloforge/plugin-sdk";
import { Bot, Braces, LayoutDashboard, RefreshCcw, Shield, Sparkles, TerminalSquare } from "lucide-react";
import { useEffect, useState } from "react";
import { BackupPanel } from "./components/BackupPanel";
import { McpPanel } from "./components/McpPanel";
import { ProviderPanel } from "./components/ProviderPanel";
import { SkillsPanel } from "./components/SkillsPanel";
import { TargetCard } from "./components/TargetCard";
import { DEFAULT_MCP_SPEC, defaultProviderForm } from "./defaults";
import { parseCcSwitchMcpUrl, parseCcSwitchProviderUrl, parseCcSwitchSkillUrl, formatError } from "./deepLinkImport";
import { useSwitchboardT } from "./i18n";
import { useSwitchboard } from "./hooks/useSwitchboard";
import type { McpAppSelection, PluginSettings, ProviderForm, SkillImportPatch, TargetStatus } from "./types";

type SwitchboardTab = "overview" | "claude" | "codex" | "mcp" | "skills" | "backups";

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
  const [claudeImportUrl, setClaudeImportUrl] = useState("");
  const [codexImportUrl, setCodexImportUrl] = useState("");
  const [mcpId, setMcpId] = useState("context7");
  const [mcpImportUrl, setMcpImportUrl] = useState("");
  const [mcpApps, setMcpApps] = useState<McpAppSelection>({
    claude: true,
    codex: true,
  });
  const [mcpSpec, setMcpSpec] = useState(DEFAULT_MCP_SPEC);
  const [skillImportUrl, setSkillImportUrl] = useState("");
  const [skillImport, setSkillImport] = useState<SkillImportPatch | null>(null);
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
    setClaudeForm((current) => ({
      ...current,
      target: "claude",
      providerId: current.providerId || settings.stableCodexProviderId || "switchboard",
      name: settings.providerName?.trim() || current.name,
    }));
    setCodexForm((current) => ({
      ...current,
      target: "codex",
      providerId: current.providerId || settings.stableCodexProviderId || "switchboard",
      name: settings.providerName?.trim() || current.name,
    }));
  }, [settings.stableCodexProviderId, settings.providerName]);

  const importProviderUrl = (target: "claude" | "codex") => {
    try {
      const currentForm = target === "claude" ? claudeForm : codexForm;
      const url = target === "claude" ? claudeImportUrl : codexImportUrl;
      const patch = parseCcSwitchProviderUrl(url, currentForm.providerId);
      const normalizedPatch = {
        ...patch,
        target,
      };
      if (target === "claude") {
        setClaudeForm((current) => ({ ...current, ...normalizedPatch }));
      } else {
        setCodexForm((current) => ({ ...current, ...normalizedPatch }));
      }
      setMessage(t("switchboard.message.providerImported"));
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
      setMessage(t("switchboard.message.mcpImported"));
    } catch (error) {
      setMessage(formatError(error));
    }
  };

  const importSkillUrl = () => {
    try {
      const patch = parseCcSwitchSkillUrl(skillImportUrl);
      setSkillImport(patch);
      setMessage(t("switchboard.message.skillImported"));
    } catch (error) {
      setMessage(formatError(error));
    }
  };

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
    { id: "skills", label: t("switchboard.tab.skills"), icon: Sparkles },
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

      <nav className="sb-tabbar" aria-label="Switchboard sections">
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
            ccswitchUrl={claudeImportUrl}
            setCcswitchUrl={setClaudeImportUrl}
            busy={busy}
            onImport={() => importProviderUrl("claude")}
            onApply={() => void applyProvider(claudeForm)}
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
            ccswitchUrl={codexImportUrl}
            setCcswitchUrl={setCodexImportUrl}
            busy={busy}
            onImport={() => importProviderUrl("codex")}
            onApply={() => void applyProvider(codexForm)}
            t={t}
          />
        </section>
      )}

      {activeTab === "mcp" && (
        <section className="sb-pane">
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
            t={t}
          />
        </section>
      )}

      {activeTab === "skills" && (
        <section className="sb-pane">
          <SkillsPanel
            skillImportUrl={skillImportUrl}
            setSkillImportUrl={setSkillImportUrl}
            skillImport={skillImport}
            onImport={importSkillUrl}
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
