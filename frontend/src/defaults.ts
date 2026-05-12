import type { PluginSettings, ProviderForm } from "./types";

export const DEFAULT_PROVIDER_NAME = "H3D Gateway";

export const DEFAULT_MCP_SPEC = `{
  "type": "stdio",
  "command": "npx",
  "args": ["-y", "@modelcontextprotocol/server-filesystem", "."]
}`;

export function defaultProviderForm(settings: PluginSettings): ProviderForm {
  return {
    target: settings.defaultTarget ?? "both",
    name: settings.providerName?.trim() || DEFAULT_PROVIDER_NAME,
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
