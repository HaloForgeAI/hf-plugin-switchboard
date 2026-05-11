import { invokePlugin, usePluginSettings, AppSelect } from "@haloforge/plugin-sdk";
import {
  CheckCircle2,
  Clipboard,
  Download,
  PlugZap,
  RefreshCcw,
  RotateCcw,
  Shield,
  TerminalSquare,
} from "lucide-react";
import { useCallback, useEffect, useState } from "react";
import type { Dispatch, SetStateAction } from "react";

type Target = "claude" | "codex" | "both";

interface PluginSettings {
  defaultTarget?: Target;
  stableCodexProviderId?: string;
}

interface PathStatus {
  label: string;
  path: string;
  exists: boolean;
}

interface TargetStatus {
  id: string;
  label: string;
  configured: boolean;
  summary?: string;
  paths: PathStatus[];
}

interface BackupFile {
  originalPath: string;
  backupFile?: string | null;
  existed: boolean;
}

interface BackupInfo {
  id: string;
  createdAt: string;
  path: string;
  files: BackupFile[];
}

interface SwitchboardStatus {
  os: string;
  homeDir?: string | null;
  dataDir: string;
  targets: TargetStatus[];
  backups: BackupInfo[];
}

interface ProviderForm {
  target: Target;
  name: string;
  baseUrl: string;
  apiKey: string;
  providerId: string;
  model: string;
  reasoningEffort: string;
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
  setClaudePrimaryApiKey: boolean;
  skipClaudeOnboarding: boolean;
}

const DEFAULT_MCP_SPEC = `{
  "type": "stdio",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
}`;

function defaultProviderForm(settings: PluginSettings): ProviderForm {
  return {
    target: settings.defaultTarget ?? "both",
    name: "Sub2API Gateway",
    baseUrl: "",
    apiKey: "",
    providerId: settings.stableCodexProviderId ?? "switchboard",
    model: "",
    reasoningEffort: "high",
    haikuModel: "",
    sonnetModel: "",
    opusModel: "",
    setClaudePrimaryApiKey: false,
    skipClaudeOnboarding: true,
  };
}

export function SwitchboardPanel() {
  const { settings } = usePluginSettings<PluginSettings>();
  const [status, setStatus] = useState<SwitchboardStatus | null>(null);
  const [form, setForm] = useState<ProviderForm>(() => defaultProviderForm({}));
  const [ccswitchUrl, setCcswitchUrl] = useState("");
  const [mcpId, setMcpId] = useState("context7");
  const [mcpApps, setMcpApps] = useState<Record<"claude" | "codex", boolean>>({
    claude: true,
    codex: true,
  });
  const [mcpSpec, setMcpSpec] = useState(DEFAULT_MCP_SPEC);
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<string | null>(null);

  useEffect(() => {
    setForm((current) => ({
      ...current,
      target: current.target || settings.defaultTarget || "both",
      providerId: current.providerId || settings.stableCodexProviderId || "switchboard",
    }));
  }, [settings.defaultTarget, settings.stableCodexProviderId]);

  const refresh = useCallback(async () => {
    const next = await invokePlugin<SwitchboardStatus>("switchboard_status", {});
    setStatus(next);
  }, []);

  useEffect(() => {
    void refresh().catch((error) => setMessage(formatError(error)));
  }, [refresh]);

  const applyProvider = async () => {
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
  };

  const importCcSwitchUrl = () => {
    try {
      const patch = parseCcSwitchUrl(ccswitchUrl, form.providerId);
      setForm((current) => ({ ...current, ...patch }));
      setMessage("Imported provider fields.");
    } catch (error) {
      setMessage(formatError(error));
    }
  };

  const installMcp = async () => {
    setBusy("mcp");
    setMessage(null);
    try {
      const spec = JSON.parse(mcpSpec);
      const apps = Object.entries(mcpApps)
        .filter(([, enabled]) => enabled)
        .map(([app]) => app);
      const result = await invokePlugin<{ changedPaths: string[] }>("switchboard_install_mcp", {
        id: mcpId,
        apps,
        spec,
      });
      setMessage(`Installed MCP into ${result.changedPaths.length} config file(s).`);
      await refresh();
    } catch (error) {
      setMessage(formatError(error));
    } finally {
      setBusy(null);
    }
  };

  const restoreBackup = async (backupId: string) => {
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
        <div className="sb-panel sb-provider-panel">
          <div className="sb-panel-title">
            <PlugZap size={18} />
            <h2>Provider</h2>
          </div>

          <div className="sb-ccswitch-row">
            <input
              value={ccswitchUrl}
              onChange={(event) => setCcswitchUrl(event.target.value)}
              placeholder="ccswitch://v1/import?resource=provider..."
              spellCheck={false}
            />
            <button type="button" className="sb-secondary-button" onClick={importCcSwitchUrl}>
              <Download size={15} />
              Import
            </button>
          </div>

          <div className="sb-form-grid">
            <label>
              <span>Target</span>
              <AppSelect
                value={form.target}
                onChange={(event) => updateForm(setForm, { target: event.target.value as Target })}
              >
                <option value="both">Claude + Codex</option>
                <option value="claude">Claude Code</option>
                <option value="codex">Codex</option>
              </AppSelect>
            </label>
            <label>
              <span>Name</span>
              <input
                value={form.name}
                onChange={(event) => updateForm(setForm, { name: event.target.value })}
              />
            </label>
            <label>
              <span>Base URL</span>
              <input
                value={form.baseUrl}
                onChange={(event) => updateForm(setForm, { baseUrl: event.target.value })}
                placeholder="https://api.example.com"
                spellCheck={false}
              />
            </label>
            <label>
              <span>API key</span>
              <input
                value={form.apiKey}
                onChange={(event) => updateForm(setForm, { apiKey: event.target.value })}
                type="password"
                spellCheck={false}
              />
            </label>
            <label>
              <span>Model</span>
              <input
                value={form.model}
                onChange={(event) => updateForm(setForm, { model: event.target.value })}
                placeholder="auto"
                spellCheck={false}
              />
            </label>
            <label>
              <span>Codex provider id</span>
              <input
                value={form.providerId}
                onChange={(event) => updateForm(setForm, { providerId: event.target.value })}
                spellCheck={false}
              />
            </label>
          </div>

          <div className="sb-options">
            <label>
              <input
                type="checkbox"
                checked={form.skipClaudeOnboarding}
                onChange={(event) => updateForm(setForm, { skipClaudeOnboarding: event.target.checked })}
              />
              <span>Claude onboarding flag</span>
            </label>
            <label>
              <input
                type="checkbox"
                checked={form.setClaudePrimaryApiKey}
                onChange={(event) => updateForm(setForm, { setClaudePrimaryApiKey: event.target.checked })}
              />
              <span>Claude primaryApiKey</span>
            </label>
          </div>

          <div className="sb-actions">
            <button
              type="button"
              className="sb-primary-button"
              disabled={busy === "provider"}
              onClick={() => void applyProvider()}
            >
              <CheckCircle2 size={16} />
              Apply
            </button>
          </div>
        </div>

        <div className="sb-panel">
          <div className="sb-panel-title">
            <TerminalSquare size={18} />
            <h2>MCP</h2>
          </div>
          <div className="sb-form-grid sb-mcp-grid">
            <label>
              <span>ID</span>
              <input value={mcpId} onChange={(event) => setMcpId(event.target.value)} spellCheck={false} />
            </label>
            <div className="sb-options sb-inline-options">
              <label>
                <input
                  type="checkbox"
                  checked={mcpApps.claude}
                  onChange={(event) => setMcpApps((current) => ({ ...current, claude: event.target.checked }))}
                />
                <span>Claude</span>
              </label>
              <label>
                <input
                  type="checkbox"
                  checked={mcpApps.codex}
                  onChange={(event) => setMcpApps((current) => ({ ...current, codex: event.target.checked }))}
                />
                <span>Codex</span>
              </label>
            </div>
          </div>
          <textarea
            className="sb-json-editor"
            value={mcpSpec}
            onChange={(event) => setMcpSpec(event.target.value)}
            spellCheck={false}
          />
          <div className="sb-actions">
            <button
              type="button"
              className="sb-primary-button"
              disabled={busy === "mcp"}
              onClick={() => void installMcp()}
            >
              <Clipboard size={16} />
              Install
            </button>
          </div>
        </div>

        <div className="sb-panel">
          <div className="sb-panel-title">
            <Shield size={18} />
            <h2>Backups</h2>
          </div>
          <div className="sb-backup-list">
            {status?.backups.length ? (
              status.backups.slice(0, 8).map((backup) => (
                <div className="sb-backup-item" key={backup.id}>
                  <div>
                    <strong>{backup.id}</strong>
                    <span>{backup.files.length} file(s)</span>
                  </div>
                  <button
                    type="button"
                    className="sb-icon-button"
                    disabled={busy === `restore:${backup.id}`}
                    onClick={() => void restoreBackup(backup.id)}
                    title="Restore"
                  >
                    <RotateCcw size={16} />
                  </button>
                </div>
              ))
            ) : (
              <div className="sb-empty">No backups</div>
            )}
          </div>
        </div>
      </section>
    </main>
  );
}

function TargetCard({ target }: { target: TargetStatus }) {
  return (
    <article className="sb-target-card">
      <div className="sb-target-head">
        <div>
          <h2>{target.label}</h2>
          {target.summary && <p>{target.summary}</p>}
        </div>
        <span className={target.configured ? "sb-badge sb-badge-on" : "sb-badge"}>
          {target.configured ? "Configured" : "Empty"}
        </span>
      </div>
      <div className="sb-path-list">
        {target.paths.map((path) => (
          <div key={`${target.id}:${path.label}`} className="sb-path-row">
            <span>{path.label}</span>
            <code>{path.path}</code>
            <i className={path.exists ? "sb-dot sb-dot-on" : "sb-dot"} />
          </div>
        ))}
      </div>
    </article>
  );
}

function updateForm(
  setForm: Dispatch<SetStateAction<ProviderForm>>,
  patch: Partial<ProviderForm>,
) {
  setForm((current) => ({ ...current, ...patch }));
}

function parseCcSwitchUrl(raw: string, fallbackProviderId: string): Partial<ProviderForm> {
  const value = raw.trim();
  if (!value) {
    throw new Error("URL is empty.");
  }

  const url = new URL(value);
  if (url.protocol !== "ccswitch:") {
    throw new Error("Expected ccswitch:// URL.");
  }

  const app = normalizeTarget(url.searchParams.get("app"));
  const resource = url.searchParams.get("resource");
  if (resource && resource !== "provider") {
    throw new Error(`Unsupported resource: ${resource}`);
  }

  const patch: Partial<ProviderForm> = {
    target: app,
    name: url.searchParams.get("name") || "Imported Provider",
    baseUrl: url.searchParams.get("endpoint") || url.searchParams.get("baseUrl") || "",
    apiKey: url.searchParams.get("apiKey") || "",
    model: url.searchParams.get("model") || "",
    providerId: fallbackProviderId || "switchboard",
  };

  const config = url.searchParams.get("config");
  if (config) {
    const merged = parseEmbeddedConfig(config, app);
    for (const [key, fieldValue] of Object.entries(merged)) {
      if (fieldValue !== undefined) {
        (patch as Record<string, unknown>)[key] = fieldValue;
      }
    }
  }

  return patch;
}

function normalizeTarget(value: string | null): Target {
  if (value === "claude") return "claude";
  if (value === "codex") return "codex";
  return "both";
}

function parseEmbeddedConfig(config: string, target: Target): Partial<ProviderForm> {
  const decoded = decodeBase64Utf8(config);
  const parsed = JSON.parse(decoded) as unknown;
  if (!parsed || typeof parsed !== "object") return {};

  const data = parsed as Record<string, unknown>;
  if (target === "codex" || target === "both") {
    const auth = data.auth as Record<string, unknown> | undefined;
    const configText = typeof data.config === "string" ? data.config : "";
    return {
      apiKey: typeof auth?.OPENAI_API_KEY === "string" ? auth.OPENAI_API_KEY : undefined,
      baseUrl: extractTomlString(configText, "base_url") ?? undefined,
      model: extractTomlString(configText, "model") ?? undefined,
      providerId: extractTomlString(configText, "model_provider") ?? undefined,
    };
  }

  const env = data.env as Record<string, unknown> | undefined;
  return {
    apiKey:
      typeof env?.ANTHROPIC_AUTH_TOKEN === "string"
        ? env.ANTHROPIC_AUTH_TOKEN
        : typeof env?.ANTHROPIC_API_KEY === "string"
          ? env.ANTHROPIC_API_KEY
          : undefined,
    baseUrl: typeof env?.ANTHROPIC_BASE_URL === "string" ? env.ANTHROPIC_BASE_URL : undefined,
    model: typeof env?.ANTHROPIC_MODEL === "string" ? env.ANTHROPIC_MODEL : undefined,
    haikuModel:
      typeof env?.ANTHROPIC_DEFAULT_HAIKU_MODEL === "string" ? env.ANTHROPIC_DEFAULT_HAIKU_MODEL : undefined,
    sonnetModel:
      typeof env?.ANTHROPIC_DEFAULT_SONNET_MODEL === "string" ? env.ANTHROPIC_DEFAULT_SONNET_MODEL : undefined,
    opusModel:
      typeof env?.ANTHROPIC_DEFAULT_OPUS_MODEL === "string" ? env.ANTHROPIC_DEFAULT_OPUS_MODEL : undefined,
  };
}

function decodeBase64Utf8(value: string) {
  const normalized = value.replace(/-/g, "+").replace(/_/g, "/");
  const binary = atob(normalized);
  const bytes = Uint8Array.from(binary, (char) => char.charCodeAt(0));
  return new TextDecoder().decode(bytes);
}

function extractTomlString(text: string, key: string) {
  const match = text.match(new RegExp(`^${key}\\s*=\\s*"([^"]*)"`, "m"));
  return match?.[1];
}

function formatError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}
