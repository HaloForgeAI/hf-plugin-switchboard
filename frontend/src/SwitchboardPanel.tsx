import { clearPendingPluginDeepLink, usePluginDeepLink, usePluginSettings } from "@haloforge/plugin-sdk";
import { Bot, Braces, LayoutDashboard, RefreshCcw, Shield, TerminalSquare } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { BackupPanel } from "./components/BackupPanel";
import { McpPanel } from "./components/McpPanel";
import { ProviderPanel } from "./components/ProviderPanel";
import { TargetCard } from "./components/TargetCard";
import { DEFAULT_CODEX_PROVIDER_ID, DEFAULT_MCP_SPEC, defaultProviderForm } from "./defaults";
import { useSwitchboardT } from "./i18n";
import { useSwitchboard } from "./hooks/useSwitchboard";
import type { McpAppSelection, PluginSettings, ProviderForm, SwitchboardImportPatch } from "./types";

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
    cleanupCodex,
    discoverModels,
    restoreBackup,
  } = useSwitchboard();

  const applyImportPatch = useCallback((patch: SwitchboardImportPatch) => {
    const providerPatch = normalizeProviderPatch(patch.provider);
    if (providerPatch) {
      const target = providerPatch.target ?? settings.defaultTarget ?? "both";
      if (target === "codex") {
        setCodexForm((current) => ({ ...current, ...providerPatch, target: "codex" }));
      } else if (target === "both") {
        setClaudeForm((current) => ({ ...current, ...providerPatch, target: "claude" }));
        setCodexForm((current) => ({ ...current, ...providerPatch, target: "codex" }));
      } else {
        setClaudeForm((current) => ({ ...current, ...providerPatch, target: "claude" }));
      }
    }

    if (patch.mcp) {
      if (typeof patch.mcp.id === "string" && patch.mcp.id.trim()) {
        setMcpId(patch.mcp.id.trim());
      }
      if (patch.mcp.apps) {
        const apps = patch.mcp.apps;
        setMcpApps((current) => ({ ...current, ...apps }));
      }
      if (typeof patch.mcp.specText === "string" && patch.mcp.specText.trim()) {
        setMcpSpec(patch.mcp.specText);
      }
    }

    setActiveTab(resolveImportTab(patch, providerPatch, settings.defaultTarget ?? "both"));
    setMessage(t("switchboard.message.importReady"));
  }, [setMessage, settings.defaultTarget, t]);

  usePluginDeepLink(useCallback((link) => {
    if (link.route !== "/v1/import" && link.route !== "/import") {
      return;
    }
    const patch = parseImportPatch(link.params);
    if (!patch) {
      setMessage(t("switchboard.message.importInvalid"));
      return;
    }
    applyImportPatch(patch);
    clearPendingPluginDeepLink();
  }, [applyImportPatch, setMessage, t]));

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
            onCleanupCodex={() => void cleanupCodex({ providerId: codexForm.providerId })}
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

function resolveImportTab(
  patch: SwitchboardImportPatch,
  providerPatch: SwitchboardImportPatch["provider"] | null,
  defaultTarget: ProviderForm["target"],
): SwitchboardTab {
  if (patch.tab) {
    return patch.tab;
  }
  if (patch.mcp && !providerPatch) {
    return "mcp";
  }
  if ((providerPatch?.target ?? defaultTarget) === "codex") {
    return "codex";
  }
  return providerPatch ? "claude" : "overview";
}

function normalizeProviderPatch(
  value: SwitchboardImportPatch["provider"],
): SwitchboardImportPatch["provider"] | null {
  if (!value || typeof value !== "object") {
    return null;
  }
  const patch: SwitchboardImportPatch["provider"] = {};
  const target = value.target;
  if (target === "claude" || target === "codex" || target === "both") {
    patch.target = target;
  }
  for (const key of ["name", "baseUrl", "apiKey", "modelsPath", "providerId", "model", "reasoningEffort", "haikuModel", "sonnetModel", "opusModel"] as const) {
    if (typeof value[key] === "string") {
      patch[key] = value[key];
    }
  }
  return Object.keys(patch).length > 0 ? patch : null;
}

function parseImportPatch(params: Record<string, string>): SwitchboardImportPatch | null {
  const encoded = params.payload ?? params.data ?? params.config;
  if (encoded) {
    const parsed = parseJsonPayload(encoded);
    if (parsed) {
      return parsed;
    }
  }

  const patch: SwitchboardImportPatch = {};
  const tab = params.tab;
  if (tab === "claude" || tab === "codex" || tab === "mcp" || tab === "backups" || tab === "overview") {
    patch.tab = tab;
  }

  const provider = normalizeProviderPatch({
    target: params.target,
    name: params.name,
    baseUrl: params.baseUrl ?? params.base_url,
    apiKey: params.apiKey ?? params.api_key,
    modelsPath: params.modelsPath ?? params.models_path,
    providerId: params.providerId ?? params.provider_id,
    model: params.model,
    reasoningEffort: params.reasoningEffort ?? params.reasoning_effort,
    haikuModel: params.haikuModel ?? params.haiku_model,
    sonnetModel: params.sonnetModel ?? params.sonnet_model,
    opusModel: params.opusModel ?? params.opus_model,
  });
  if (provider) {
    patch.provider = provider;
  }

  const mcpSpecText = params.mcpSpec ?? params.mcp_spec ?? params.spec;
  const mcpId = params.mcpId ?? params.mcp_id;
  if (mcpId || mcpSpecText) {
    patch.mcp = {
      id: mcpId,
      specText: mcpSpecText,
      apps: {
        claude: parseBooleanParam(params.claude, true),
        codex: parseBooleanParam(params.codex, true),
      },
    };
  }

  return patch.provider || patch.mcp || patch.tab ? patch : null;
}

function parseJsonPayload(value: string): SwitchboardImportPatch | null {
  for (const candidate of [value, decodeBase64Url(value)]) {
    if (!candidate) {
      continue;
    }
    try {
      const parsed = JSON.parse(candidate) as SwitchboardImportPatch;
      return parsed && typeof parsed === "object" ? parsed : null;
    } catch {
      // Try the next encoding.
    }
  }
  return null;
}

function decodeBase64Url(value: string): string | null {
  try {
    const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
    const padded = normalized.padEnd(Math.ceil(normalized.length / 4) * 4, "=");
    const bytes = Uint8Array.from(window.atob(padded), (char) => char.charCodeAt(0));
    return new TextDecoder().decode(bytes);
  } catch {
    return null;
  }
}

function parseBooleanParam(value: string | undefined, fallback: boolean): boolean {
  if (value === undefined) {
    return fallback;
  }
  return value === "1" || value.toLowerCase() === "true" || value.toLowerCase() === "yes";
}
