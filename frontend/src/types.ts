import type { Dispatch, SetStateAction } from "react";

export type Target = "claude" | "codex" | "both";
export type BusyState = string | null;

export interface PluginSettings {
  defaultTarget?: Target;
  stableCodexProviderId?: string;
  providerName?: string;
  defaultModel?: string;
  modelsPath?: string;
}

export interface PathStatus {
  label: string;
  path: string;
  exists: boolean;
}

export interface TargetStatus {
  id: string;
  label: string;
  configured: boolean;
  summary?: string;
  details: TargetDetail[];
  paths: PathStatus[];
}

export interface TargetDetail {
  label: string;
  value: string;
  secret?: string;
}

export interface BackupFile {
  originalPath: string;
  backupFile?: string | null;
  existed: boolean;
  byteCount?: number | null;
  preview?: string | null;
}

export interface BackupInfo {
  id: string;
  createdAt: string;
  path: string;
  files: BackupFile[];
}

export interface SwitchboardStatus {
  os: string;
  homeDir?: string | null;
  dataDir: string;
  targets: TargetStatus[];
  backups: BackupInfo[];
}

export interface ProviderForm {
  target: Target;
  name: string;
  baseUrl: string;
  apiKey: string;
  modelsPath: string;
  providerId: string;
  model: string;
  reasoningEffort: string;
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
  setClaudePrimaryApiKey: boolean;
  skipClaudeOnboarding: boolean;
  enableCodexBuiltinPlugins: boolean;
  preserveCodexChatgptAuth: boolean;
}

export type SetProviderForm = Dispatch<SetStateAction<ProviderForm>>;

export interface CleanupCodexForm {
  providerId: string;
}

export interface McpAppSelection {
  claude: boolean;
  codex: boolean;
}

export interface McpImportPatch {
  id: string;
  apps: McpAppSelection;
  specText: string;
}
