import { DatabaseZap, Search, Wrench } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { BusyState, CodexLogFixStatus } from "../types";

interface CodexToolsPanelProps {
  status: CodexLogFixStatus | null;
  busy: BusyState;
  onCheck: () => void;
  onApply: () => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function CodexToolsPanel({
  status,
  busy,
  onCheck,
  onApply,
  t,
}: CodexToolsPanelProps) {
  const isBusy = busy === "codex-log-fix-check" || busy === "codex-log-fix-apply";
  const isApplied = status?.status === "applied";

  return (
    <section className="sb-panel sb-tool-panel">
      <div className="sb-section-heading">
        <div>
          <h2>{t("switchboard.codexTools.title")}</h2>
          <p>{t("switchboard.codexTools.subtitle")}</p>
        </div>
        <span className={statusChipClass(status?.status)}>
          {status ? statusLabel(status.status, t) : t("switchboard.codexTools.status.unknown")}
        </span>
      </div>

      <div className="sb-tool-row">
        <div className="sb-tool-title-row">
          <span className="sb-tab-icon-badge" aria-hidden="true">
            <DatabaseZap size={17} />
          </span>
          <div className="sb-tool-copy">
            <strong>{t("switchboard.codexTools.sqliteTitle")}</strong>
            <span>{t("switchboard.codexTools.sqliteBody")}</span>
          </div>
        </div>
        <div className="sb-actions">
          <button
            type="button"
            className="sb-secondary-button"
            disabled={isBusy}
            onClick={onCheck}
          >
            <Search size={15} />
            {t("switchboard.codexTools.check")}
          </button>
          <button
            type="button"
            className="sb-primary-button"
            disabled={isBusy || isApplied || (status?.status === "unsupported")}
            onClick={onApply}
          >
            <Wrench size={15} />
            {t("switchboard.codexTools.apply")}
          </button>
        </div>
      </div>

      {status && (
        <div className="sb-tool-result">
          <div className="sb-import-grid">
            <div className="sb-import-row">
              <span>{t("switchboard.codexTools.database")}</span>
              <code>{status.databasePath ?? "-"}</code>
            </div>
            <div className="sb-import-row">
              <span>{t("switchboard.codexTools.trigger")}</span>
              <code>{status.triggerName}</code>
            </div>
            <div className="sb-import-row sb-import-row-wide">
              <span>{t("switchboard.common.status")}</span>
              <code>{status.message}</code>
            </div>
          </div>
          <details className="sb-details sb-candidate-details">
            <summary>
              <div className="sb-details-copy">
                <strong>{t("switchboard.codexTools.candidates")}</strong>
                <span>{t("switchboard.codexTools.candidatesHint")}</span>
              </div>
            </summary>
            <div className="sb-candidate-list">
              {status.candidatePaths.map((path) => (
                <code key={path}>{path}</code>
              ))}
            </div>
          </details>
        </div>
      )}
    </section>
  );
}

function statusLabel(
  status: string,
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string,
) {
  switch (status) {
    case "applied":
      return t("switchboard.codexTools.status.applied");
    case "ready":
      return t("switchboard.codexTools.status.ready");
    case "not_found":
      return t("switchboard.codexTools.status.notFound");
    case "unsupported":
      return t("switchboard.codexTools.status.unsupported");
    default:
      return t("switchboard.codexTools.status.unknown");
  }
}

function statusChipClass(status: string | undefined) {
  if (status === "applied") return "sb-status-chip sb-status-chip-on";
  if (status === "ready") return "sb-status-chip sb-status-chip-warn";
  if (status === "unsupported") return "sb-status-chip sb-status-chip-danger";
  return "sb-status-chip";
}
