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
    codexSessionAudit,
    busy,
    message,
    setMessage,
    refresh,
    applyProvider,
    installMcp,
    cleanupCodex,
    checkCodexSessions,
    repairCodexSessions,
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
            audit={codexSessionAudit}
            busy={busy}
            onCheck={() => void checkCodexSessions()}
            onRepair={() => void repairCodexSessions()}
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
    { label: t("switchboard.provider.codexAuthMode"), value: providerPatch.codexAuthMode },
    { label: t("switchboard.provider.codexEnvKey"), value: providerPatch.codexEnvKey },
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
  for (const key of ["name", "baseUrl", "apiKey", "modelsPath", "providerId", "model", "reasoningEffort", "haikuModel", "sonnetModel", "opusModel", "codexEnvKey"] as const) {
    if (typeof value[key] === "string") {
      patch[key] = value[key];
    }
  }
  if (
    value.codexAuthMode === "api_key" ||
    value.codexAuthMode === "provider_token" ||
    value.codexAuthMode === "env_key"
  ) {
    patch.codexAuthMode = value.codexAuthMode;
  }
  return Object.keys(patch).length > 0 ? patch : null;
}

function parseImportPatch(params: Record<string, string>): SwitchboardImportPatch | null {
  const encoded = params.payload ?? params.data ?? params.config;
  if (encoded) {
    const parsed = parseJsonPayload(encoded, params);
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
    target: parseProviderTarget(params.target ?? params.app),
    name: params.name,
    baseUrl: params.baseUrl ?? params.base_url ?? params.endpoint,
    apiKey: params.apiKey ?? params.api_key,
    modelsPath: params.modelsPath ?? params.models_path,
    providerId: params.providerId ?? params.provider_id,
    model: params.model,
    reasoningEffort: params.reasoningEffort ?? params.reasoning_effort,
    haikuModel: params.haikuModel ?? params.haiku_model,
    sonnetModel: params.sonnetModel ?? params.sonnet_model,
    opusModel: params.opusModel ?? params.opus_model,
    codexAuthMode: params.codexAuthMode ?? params.codex_auth_mode,
    codexEnvKey: params.codexEnvKey ?? params.codex_env_key ?? params.env_key,
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

function parseJsonPayload(value: string, params: Record<string, string>): SwitchboardImportPatch | null {
  for (const candidate of [value, decodeBase64Url(value)]) {
    if (!candidate) {
      continue;
    }
    try {
      const parsed = JSON.parse(candidate);
      if (!parsed || typeof parsed !== "object") {
        return null;
      }
      return normalizeExternalImportPayload(parsed, params) ?? parsed as SwitchboardImportPatch;
    } catch {
      // Try the next encoding.
    }
  }
  return null;
}

function normalizeExternalImportPayload(
  parsed: Record<string, unknown>,
  params: Record<string, string>,
): SwitchboardImportPatch | null {
  const codex = normalizeCodexConfigImport(parsed, params);
  if (codex) {
    return { provider: codex, tab: "codex" };
  }
  const claude = normalizeClaudeConfigImport(parsed, params);
  if (claude) {
    return { provider: claude, tab: "claude" };
  }
  return null;
}

function normalizeCodexConfigImport(
  parsed: Record<string, unknown>,
  params: Record<string, string>,
): SwitchboardImportPatch["provider"] | null {
  if (typeof parsed.config !== "string" && typeof parsed.auth !== "object") {
    return null;
  }
  const config = typeof parsed.config === "string" ? parsed.config : "";
  const auth = parsed.auth && typeof parsed.auth === "object" ? parsed.auth as Record<string, unknown> : {};
  const providerId = tomlString(config, "model_provider") ?? params.providerId ?? params.provider_id ?? "custom";
  const providerBlock = providerId ? tomlProviderBlock(config, providerId) : "";
  const bearerToken = tomlString(providerBlock, "experimental_bearer_token");
  const envKey = tomlString(providerBlock, "env_key");
  const apiKey = stringValue(auth.OPENAI_API_KEY) ?? bearerToken ?? "";
  return normalizeProviderPatch({
    target: "codex",
    name: tomlString(providerBlock, "name") ?? params.name,
    baseUrl: tomlString(providerBlock, "base_url") ?? tomlString(config, "openai_base_url") ?? params.endpoint,
    apiKey,
    providerId,
    model: tomlString(config, "model") ?? params.model,
    reasoningEffort: tomlString(config, "model_reasoning_effort") ?? params.reasoning_effort,
    codexAuthMode: envKey ? "env_key" : bearerToken ? "provider_token" : "api_key",
    codexEnvKey: envKey ?? "",
  });
}

function normalizeClaudeConfigImport(
  parsed: Record<string, unknown>,
  params: Record<string, string>,
): SwitchboardImportPatch["provider"] | null {
  const env = parsed.env && typeof parsed.env === "object" ? parsed.env as Record<string, unknown> : null;
  if (!env) {
    return null;
  }
  return normalizeProviderPatch({
    target: "claude",
    name: params.name,
    baseUrl: stringValue(env.ANTHROPIC_BASE_URL),
    apiKey: stringValue(env.ANTHROPIC_AUTH_TOKEN) ?? stringValue(env.ANTHROPIC_API_KEY),
    model: stringValue(env.ANTHROPIC_MODEL),
    haikuModel: stringValue(env.ANTHROPIC_DEFAULT_HAIKU_MODEL),
    sonnetModel: stringValue(env.ANTHROPIC_DEFAULT_SONNET_MODEL),
    opusModel: stringValue(env.ANTHROPIC_DEFAULT_OPUS_MODEL),
  });
}

function stringValue(value: unknown): string | undefined {
  return typeof value === "string" ? value : undefined;
}

function tomlString(text: string, key: string): string | undefined {
  const escapedKey = key.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = text.match(new RegExp(`^\\s*${escapedKey}\\s*=\\s*"((?:\\\\.|[^"])*)"`, "m"));
  if (!match) {
    return undefined;
  }
  try {
    return JSON.parse(`"${match[1]}"`) as string;
  } catch {
    return match[1].replace(/\\"/g, '"');
  }
}

function tomlProviderBlock(text: string, providerId: string): string {
  const escaped = providerId.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = text.match(new RegExp(`^\\s*\\[model_providers\\.${escaped}\\]\\s*$`, "m"));
  if (!match || match.index === undefined) {
    return "";
  }
  const rest = text.slice(match.index + match[0].length);
  const nextSection = rest.search(/^\s*\[/m);
  return nextSection >= 0 ? rest.slice(0, nextSection) : rest;
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
