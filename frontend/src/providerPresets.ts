import type { CodexAuthMode, ProviderForm, Target } from "./types";

export interface ProviderPreset {
  id: string;
  label: string;
  targets: Array<Exclude<Target, "both">>;
  patch: Partial<ProviderForm> & {
    codexAuthMode?: CodexAuthMode;
  };
}

export const PROVIDER_PRESETS: ProviderPreset[] = [
  {
    id: "openai-official",
    label: "OpenAI Official",
    targets: ["codex"],
    patch: {
      name: "OpenAI Official",
      baseUrl: "https://api.openai.com/v1",
      providerId: "openai",
      model: "gpt-5.5",
      reasoningEffort: "high",
      codexAuthMode: "api_key",
      codexEnvKey: "",
    },
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    targets: ["codex"],
    patch: {
      name: "OpenRouter",
      baseUrl: "https://openrouter.ai/api/v1",
      providerId: "openrouter",
      model: "",
      reasoningEffort: "high",
      codexAuthMode: "env_key",
      codexEnvKey: "OPENROUTER_API_KEY",
    },
  },
  {
    id: "deepseek",
    label: "DeepSeek",
    targets: ["codex"],
    patch: {
      name: "DeepSeek",
      baseUrl: "https://api.deepseek.com/v1",
      providerId: "deepseek",
      model: "",
      reasoningEffort: "high",
      codexAuthMode: "env_key",
      codexEnvKey: "DEEPSEEK_API_KEY",
    },
  },
  {
    id: "siliconflow",
    label: "SiliconFlow",
    targets: ["codex"],
    patch: {
      name: "SiliconFlow",
      baseUrl: "https://api.siliconflow.cn/v1",
      providerId: "siliconflow",
      model: "",
      reasoningEffort: "high",
      codexAuthMode: "env_key",
      codexEnvKey: "SILICONFLOW_API_KEY",
    },
  },
  {
    id: "anthropic-official",
    label: "Anthropic Official",
    targets: ["claude"],
    patch: {
      name: "Anthropic Official",
      baseUrl: "https://api.anthropic.com",
      model: "claude-sonnet-4-6",
    },
  },
  {
    id: "custom-openai-compatible",
    label: "Custom OpenAI-compatible",
    targets: ["codex"],
    patch: {
      name: "Custom Provider",
      baseUrl: "",
      providerId: "custom",
      model: "",
      reasoningEffort: "high",
      codexAuthMode: "provider_token",
      codexEnvKey: "",
    },
  },
];

export function presetsForTarget(target: Exclude<Target, "both">) {
  return PROVIDER_PRESETS.filter((preset) => preset.targets.includes(target));
}
