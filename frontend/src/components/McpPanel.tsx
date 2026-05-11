import { Clipboard, Download, TerminalSquare } from "lucide-react";
import type { McpAppSelection } from "../types";

interface McpPanelProps {
  mcpId: string;
  setMcpId: (value: string) => void;
  mcpImportUrl: string;
  setMcpImportUrl: (value: string) => void;
  mcpApps: McpAppSelection;
  setMcpApps: (updater: (current: McpAppSelection) => McpAppSelection) => void;
  mcpSpec: string;
  setMcpSpec: (value: string) => void;
  busy: string | null;
  onImport: () => void;
  onInstall: () => void;
}

export function McpPanel({
  mcpId,
  setMcpId,
  mcpImportUrl,
  setMcpImportUrl,
  mcpApps,
  setMcpApps,
  mcpSpec,
  setMcpSpec,
  busy,
  onImport,
  onInstall,
}: McpPanelProps) {
  return (
    <div className="sb-panel">
      <div className="sb-panel-title">
        <TerminalSquare size={18} />
        <h2>MCP</h2>
      </div>
      <div className="sb-ccswitch-row">
        <input
          value={mcpImportUrl}
          onChange={(event) => setMcpImportUrl(event.target.value)}
          placeholder="ccswitch://v1/import?resource=mcp..."
          spellCheck={false}
        />
        <button type="button" className="sb-secondary-button" onClick={onImport}>
          <Download size={15} />
          Import
        </button>
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
        <button type="button" className="sb-primary-button" disabled={busy === "mcp"} onClick={onInstall}>
          <Clipboard size={16} />
          Install
        </button>
      </div>
    </div>
  );
}
