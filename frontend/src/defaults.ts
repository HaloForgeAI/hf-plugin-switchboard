import type { PluginSettings, ProviderForm } from "./types";

export const DEFAULT_PROVIDER_NAME = "OpenAI Official";
export const DEFAULT_CODEX_PROVIDER_ID = "openai";

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
    modelsPath: settings.modelsPath?.trim() || "/models",
    providerId: settings.stableCodexProviderId ?? DEFAULT_CODEX_PROVIDER_ID,
    model: settings.defaultModel?.trim() || "",
    reasoningEffort: "high",
    haikuModel: "",
    sonnetModel: "",
    opusModel: "",
    setClaudePrimaryApiKey: false,
    skipClaudeOnboarding: true,
    enableCodexBuiltinPlugins: true,
    preserveCodexChatgptAuth: false,
  };
}
