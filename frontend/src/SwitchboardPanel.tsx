import { clearPendingPluginDeepLink, usePluginDeepLink, usePluginSettings, type PluginDeepLink } from "@haloforge/plugin-sdk";
import { Bot, Braces, CheckCircle2, KeyRound, LayoutDashboard, RefreshCcw, Shield, TerminalSquare, X } from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import { BackupPanel } from "./components/BackupPanel";
import { CodexToolsPanel } from "./components/CodexToolsPanel";
import { McpPanel } from "./components/McpPanel";
import { ProviderPanel } from "./components/ProviderPanel";
import { TargetCard } from "./components/TargetCard";
import { DEFAULT_CODEX_PROVIDER_ID, DEFAULT_MCP_SPEC, defaultProviderForm } from "./defaults";
import { useSwitchboardT, type SwitchboardTranslationKey } from "./i18n";
import { useSwitchboard } from "./hooks/useSwitchboard";
import type { McpAppSelection, PluginSettings, ProviderForm, SwitchboardImportPatch } from "./types";

type SwitchboardTab = "overview" | "claude" | "codex" | "mcp" | "backups";

interface PendingImport {
  patch: SwitchboardImportPatch;
  providerPatch: SwitchboardImportPatch["provider"] | null;
}

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
  const [pendingImport, setPendingImport] = useState<PendingImport | null>(null);
  const {
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

  usePluginDeepLink(useCallback((link: PluginDeepLink) => {
    if (link.route !== "/v1/import" && link.route !== "/import") {
      return;
    }
    const patch = parseImportPatch(link.params);
    if (!patch) {
      setMessage(t("switchboard.message.importInvalid"));
      return;
    }
    const providerPatch = normalizeProviderPatch(patch.provider);
    const targetTab = resolveImportTab(patch, providerPatch, settings.defaultTarget ?? "both");
    setActiveTab(targetTab);
    setPendingImport({ patch, providerPatch });
    setMessage(null);
    clearPendingPluginDeepLink();
  }, [setMessage, settings.defaultTarget, t]));

  const confirmPendingImport = useCallback(() => {
    if (!pendingImport) {
      return;
    }
    applyImportPatch(pendingImport.patch);
    setPendingImport(null);
  }, [applyImportPatch, pendingImport]);

  const cancelPendingImport = useCallback(() => {
    setPendingImport(null);
  }, []);

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
          <CodexToolsPanel
            status={codexLogFixStatus}
            busy={busy}
            onCheck={() => void checkCodexLogFix()}
            onApply={() => void applyCodexLogFix()}
            t={t}
          />
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

      {pendingImport && (
        <ImportPreviewDialog
          pendingImport={pendingImport}
          defaultTarget={settings.defaultTarget ?? "both"}
          onCancel={cancelPendingImport}
          onConfirm={confirmPendingImport}
          t={t}
        />
      )}
    </main>
  );
}

interface ImportPreviewDialogProps {
  pendingImport: PendingImport;
  defaultTarget: ProviderForm["target"];
  onCancel: () => void;
  onConfirm: () => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

function ImportPreviewDialog({
  pendingImport,
  defaultTarget,
  onCancel,
  onConfirm,
  t,
}: ImportPreviewDialogProps) {
  const { patch, providerPatch } = pendingImport;
  const target = providerPatch?.target ?? defaultTarget;
  const targetLabel = target === "codex"
    ? t("switchboard.tab.codex")
    : target === "claude"
      ? t("switchboard.tab.claude")
      : t("switchboard.import.targetBoth");
  const providerRows = buildImportProviderRows(providerPatch, t);
  const hasMcp = Boolean(patch.mcp);
  const mcpTargets = patch.mcp?.apps
    ? [
        patch.mcp.apps.claude ? t("switchboard.tab.claude") : null,
        patch.mcp.apps.codex ? t("switchboard.tab.codex") : null,
      ].filter(Boolean).join(", ")
    : t("switchboard.import.targetBoth");

  return (
    <div className="sb-modal-backdrop" role="presentation">
      <section
        aria-labelledby="switchboard-import-title"
        aria-modal="true"
        className="sb-import-dialog"
        role="dialog"
      >
        <div className="sb-import-head">
          <div className="sb-import-title-row">
            <span className="sb-import-icon" aria-hidden="true">
              <KeyRound size={18} />
            </span>
            <div>
              <h2 id="switchboard-import-title">{t("switchboard.import.title")}</h2>
              <p>{t("switchboard.import.subtitle", { target: targetLabel })}</p>
            </div>
          </div>
          <button className="sb-mini-icon-button" type="button" onClick={onCancel} title={t("switchboard.import.cancel")}>
            <X size={14} />
          </button>
        </div>

        {providerPatch && (
          <div className="sb-import-section">
            <div className="sb-import-section-head">
              <strong>{t("switchboard.import.providerTitle")}</strong>
              <span className="sb-status-chip sb-status-chip-on">{targetLabel}</span>
            </div>
            <div className="sb-import-grid">
              {providerRows.map((row) => (
                <div className="sb-import-row" key={row.label}>
                  <span>{row.label}</span>
                  <code>{row.value}</code>
                </div>
              ))}
            </div>
          </div>
        )}

        {hasMcp && (
          <div className="sb-import-section">
            <div className="sb-import-section-head">
              <strong>{t("switchboard.import.mcpTitle")}</strong>
              <span className="sb-status-chip">{mcpTargets}</span>
            </div>
            <div className="sb-import-grid">
              {patch.mcp?.id && (
                <div className="sb-import-row">
                  <span>{t("switchboard.mcp.id")}</span>
                  <code>{patch.mcp.id}</code>
                </div>
              )}
              {patch.mcp?.specText && (
                <div className="sb-import-row sb-import-row-wide">
                  <span>{t("switchboard.mcp.spec")}</span>
                  <code>{compactPreview(patch.mcp.specText)}</code>
                </div>
              )}
            </div>
          </div>
        )}

        <p className="sb-import-note">{t("switchboard.import.note")}</p>

        <div className="sb-import-actions">
          <button className="sb-secondary-button" type="button" onClick={onCancel}>
            {t("switchboard.import.cancel")}
          </button>
          <button className="sb-primary-button" type="button" onClick={onConfirm}>
            <CheckCircle2 size={16} />
            {t("switchboard.import.confirm")}
          </button>
        </div>
      </section>
    </div>
  );
}

function buildImportProviderRows(
  providerPatch: SwitchboardImportPatch["provider"] | null,
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string,
) {
  if (!providerPatch) {
    return [];
  }
  return [
    { label: t("switchboard.provider.name"), value: providerPatch.name },
    { label: t("switchboard.provider.baseUrl"), value: providerPatch.baseUrl },
    { label: t("switchboard.provider.apiKey"), value: maskSecret(providerPatch.apiKey) },
    { label: t("switchboard.provider.model"), value: providerPatch.model },
    { label: t("switchboard.provider.modelsPath"), value: providerPatch.modelsPath },
    { label: t("switchboard.provider.providerId"), value: providerPatch.providerId },
    { label: t("switchboard.provider.reasoning"), value: providerPatch.reasoningEffort },
  ].filter((row): row is { label: string; value: string } => typeof row.value === "string" && row.value.trim().length > 0);
}

function maskSecret(value: string | undefined): string | undefined {
  if (!value) {
    return value;
  }
  const trimmed = value.trim();
  if (trimmed.length <= 8) {
    return "****";
  }
  return `${trimmed.slice(0, 4)}****${trimmed.slice(-4)}`;
}

function compactPreview(value: string): string {
  return value.replace(/\s+/g, " ").trim();
}

function parseProviderTarget(value: string | undefined): ProviderForm["target"] | undefined {
  return value === "claude" || value === "codex" || value === "both" ? value : undefined;
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
    target: parseProviderTarget(params.target),
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
