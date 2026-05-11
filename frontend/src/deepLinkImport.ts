import type { McpAppSelection, McpImportPatch, ProviderForm, Target } from "./types";

export function parseCcSwitchProviderUrl(raw: string, fallbackProviderId: string): Partial<ProviderForm> {
  const url = parseCcSwitchUrl(raw);
  const resource = url.searchParams.get("resource");
  if (resource && resource !== "provider") {
    throw new Error(`Unsupported provider resource: ${resource}`);
  }

  const app = normalizeTarget(url.searchParams.get("app"));
  const endpoint = firstCsvValue(url.searchParams.get("endpoint") || url.searchParams.get("baseUrl") || "");
  const patch: Partial<ProviderForm> = {
    target: app,
    name: url.searchParams.get("name") || "Imported Provider",
    baseUrl: endpoint,
    apiKey: url.searchParams.get("apiKey") || "",
    model: url.searchParams.get("model") || "",
    providerId: fallbackProviderId || "switchboard",
    haikuModel: url.searchParams.get("haikuModel") || "",
    sonnetModel: url.searchParams.get("sonnetModel") || "",
    opusModel: url.searchParams.get("opusModel") || "",
  };

  const config = url.searchParams.get("config");
  if (config) {
    const merged = parseEmbeddedProviderConfig(config, app);
    for (const [key, fieldValue] of Object.entries(merged)) {
      if (fieldValue !== undefined) {
        (patch as Record<string, unknown>)[key] = fieldValue;
      }
    }
  }

  return patch;
}

export function parseCcSwitchMcpUrl(raw: string): McpImportPatch {
  const url = parseCcSwitchUrl(raw);
  const resource = url.searchParams.get("resource");
  if (resource !== "mcp") {
    throw new Error(`Expected resource=mcp, received ${resource || "empty"}.`);
  }

  const name = url.searchParams.get("name") || "mcp-server";
  const config = url.searchParams.get("config");
  if (!config) {
    throw new Error("MCP import URL requires config.");
  }

  return {
    id: name,
    apps: parseMcpApps(url.searchParams.get("apps") || url.searchParams.get("app") || ""),
    specText: JSON.stringify(parseMcpConfig(config), null, 2),
  };
}

export function formatError(error: unknown) {
  if (error instanceof Error) return error.message;
  if (typeof error === "string") return error;
  return JSON.stringify(error);
}

function parseCcSwitchUrl(raw: string) {
  const value = raw.trim();
  if (!value) {
    throw new Error("URL is empty.");
  }

  const url = new URL(value);
  if (url.protocol !== "ccswitch:") {
    throw new Error("Expected ccswitch:// URL.");
  }
  return url;
}

function normalizeTarget(value: string | null): Target {
  if (value === "claude" || value === "claude-code" || value === "claudecode") return "claude";
  if (value === "codex") return "codex";
  return "both";
}

function parseMcpApps(value: string): McpAppSelection {
  const apps = value
    .split(",")
    .map((app) => app.trim().toLowerCase())
    .filter(Boolean);
  const selection = {
    claude:
      apps.length === 0 ||
      apps.includes("both") ||
      apps.some((app) => app === "claude" || app === "claude-code" || app === "claudecode"),
    codex: apps.length === 0 || apps.includes("both") || apps.includes("codex"),
  };
  if (!selection.claude && !selection.codex) {
    throw new Error("MCP link does not target Claude Code or Codex.");
  }
  return selection;
}

function parseEmbeddedProviderConfig(config: string, target: Target): Partial<ProviderForm> {
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

function parseMcpConfig(config: string) {
  try {
    return JSON.parse(config) as unknown;
  } catch {
    return JSON.parse(decodeBase64Utf8(config)) as unknown;
  }
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

function firstCsvValue(value: string) {
  return value.split(",")[0]?.trim() || "";
}
