import type { Dispatch, SetStateAction } from "react";

export type Target = "claude" | "codex" | "both";
export type BusyState = string | null;

export interface PluginSettings {
  defaultTarget?: Target;
  stableCodexProviderId?: string;
  providerName?: string;
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
  paths: PathStatus[];
}

export interface BackupFile {
  originalPath: string;
  backupFile?: string | null;
  existed: boolean;
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
  providerId: string;
  model: string;
  reasoningEffort: string;
  haikuModel: string;
  sonnetModel: string;
  opusModel: string;
  setClaudePrimaryApiKey: boolean;
  skipClaudeOnboarding: boolean;
}

export type SetProviderForm = Dispatch<SetStateAction<ProviderForm>>;

export interface McpAppSelection {
  claude: boolean;
  codex: boolean;
}

export interface McpImportPatch {
  id: string;
  apps: McpAppSelection;
  specText: string;
}

export interface SkillImportPatch {
  name: string;
  app: "claude" | "codex" | "gemini" | "all";
  repo: string;
  directory: string;
  branch: string;
}
