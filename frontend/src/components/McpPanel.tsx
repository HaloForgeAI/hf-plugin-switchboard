import { Clipboard, TerminalSquare } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { McpAppSelection } from "../types";

interface McpPanelProps {
  mcpId: string;
  setMcpId: (value: string) => void;
  mcpApps: McpAppSelection;
  setMcpApps: (updater: (current: McpAppSelection) => McpAppSelection) => void;
  mcpSpec: string;
  setMcpSpec: (value: string) => void;
  busy: string | null;
  onInstall: () => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function McpPanel({
  mcpId,
  setMcpId,
  mcpApps,
  setMcpApps,
  mcpSpec,
  setMcpSpec,
  busy,
  onInstall,
  t,
}: McpPanelProps) {
  return (
    <section className="sb-panel">
      <div className="sb-section-heading">
        <div>
          <h2>{t("switchboard.mcp.title")}</h2>
          <p>{t("switchboard.mcp.subtitle")}</p>
        </div>
        <span className="sb-tab-icon-badge">
          <TerminalSquare size={16} />
        </span>
      </div>

      <div className="sb-form-grid sb-mcp-fields">
        <label>
          <span>{t("switchboard.mcp.id")}</span>
          <input value={mcpId} onChange={(event) => setMcpId(event.target.value)} spellCheck={false} />
        </label>
        <fieldset className="sb-checkbox-group">
          <legend>{t("switchboard.mcp.apps")}</legend>
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
        </fieldset>
      </div>

      <label className="sb-block-field">
        <span>{t("switchboard.mcp.spec")}</span>
        <textarea
          className="sb-json-editor"
          value={mcpSpec}
          onChange={(event) => setMcpSpec(event.target.value)}
          spellCheck={false}
        />
      </label>

      <div className="sb-actions">
        <button type="button" className="sb-primary-button" disabled={busy === "mcp"} onClick={onInstall}>
          <Clipboard size={16} />
          {t("switchboard.mcp.install")}
        </button>
      </div>
    </section>
  );
}
