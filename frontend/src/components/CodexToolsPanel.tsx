import { History, Search, Wrench } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { BusyState, CodexSessionAudit } from "../types";

interface CodexToolsPanelProps {
  audit: CodexSessionAudit | null;
  busy: BusyState;
  onCheck: () => void;
  onRepair: () => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function CodexToolsPanel({
  audit,
  busy,
  onCheck,
  onRepair,
  t,
}: CodexToolsPanelProps) {
  const isBusy = busy === "codex-session-audit" || busy === "codex-session-repair";
  const needsRepair = Boolean(
    audit &&
      (audit.hiddenSessionCandidates > 0 ||
        audit.stateThreadOtherProvider > 0 ||
        audit.stateThreadMissingProvider > 0),
  );

  return (
    <section className="sb-panel sb-tool-panel">
      <div className="sb-section-heading">
        <div>
          <h2>{t("switchboard.codexTools.title")}</h2>
          <p>{t("switchboard.codexTools.subtitle")}</p>
        </div>
        <span className={statusChipClass(audit, needsRepair)}>
          {audit
            ? needsRepair
              ? t("switchboard.codexTools.status.needsRepair")
              : t("switchboard.codexTools.status.ok")
            : t("switchboard.codexTools.status.unknown")}
        </span>
      </div>

      <div className="sb-tool-row">
        <div className="sb-tool-title-row">
          <span className="sb-tab-icon-badge" aria-hidden="true">
            <History size={17} />
          </span>
          <div className="sb-tool-copy">
            <strong>{t("switchboard.codexTools.sessionsTitle")}</strong>
            <span>{t("switchboard.codexTools.sessionsBody")}</span>
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
            disabled={isBusy || !needsRepair}
            onClick={onRepair}
          >
            <Wrench size={15} />
            {t("switchboard.codexTools.repair")}
          </button>
        </div>
      </div>

      {audit && (
        <div className="sb-tool-result">
          <div className="sb-import-grid">
            <MetricRow label={t("switchboard.codexTools.currentProvider")} value={audit.currentProvider} />
            <MetricRow label={t("switchboard.codexTools.hiddenSessions")} value={String(audit.hiddenSessionCandidates)} />
            <MetricRow label={t("switchboard.codexTools.sessionFiles")} value={String(audit.sessionFiles)} />
            <MetricRow label={t("switchboard.codexTools.archivedFiles")} value={String(audit.archivedSessionFiles)} />
            <MetricRow label={t("switchboard.codexTools.indexedSessions")} value={String(audit.indexedSessions)} />
            <MetricRow label={t("switchboard.codexTools.sqliteThreads")} value={String(audit.stateThreadRows)} />
            <MetricRow
              label={t("switchboard.codexTools.sqliteOther")}
              value={String(audit.stateThreadOtherProvider + audit.stateThreadMissingProvider)}
            />
            <MetricRow label={t("switchboard.codexTools.codexHome")} value={audit.codexHome} wide />
            <MetricRow label={t("switchboard.codexTools.database")} value={audit.stateDatabasePath ?? "-"} wide />
          </div>

          {audit.providerCounts.length > 0 && (
            <details className="sb-details sb-candidate-details">
              <summary>
                <div className="sb-details-copy">
                  <strong>{t("switchboard.codexTools.providerBuckets")}</strong>
                  <span>{t("switchboard.codexTools.providerBucketsHint")}</span>
                </div>
              </summary>
              <div className="sb-candidate-list">
                {audit.providerCounts.map((item) => (
                  <code key={item.provider}>
                    {item.provider}: {item.count}
                    {item.current ? ` ${t("switchboard.codexTools.currentMarker")}` : ""}
                  </code>
                ))}
              </div>
            </details>
          )}

          {audit.warnings.length > 0 && (
            <details className="sb-details sb-candidate-details">
              <summary>
                <div className="sb-details-copy">
                  <strong>{t("switchboard.codexTools.warnings")}</strong>
                  <span>{t("switchboard.codexTools.warningsHint")}</span>
                </div>
              </summary>
              <div className="sb-candidate-list">
                {audit.warnings.map((warning) => (
                  <code key={warning}>{warning}</code>
                ))}
              </div>
            </details>
          )}
        </div>
      )}
    </section>
  );
}

function MetricRow({
  label,
  value,
  wide,
}: {
  label: string;
  value: string;
  wide?: boolean;
}) {
  return (
    <div className={wide ? "sb-import-row sb-import-row-wide" : "sb-import-row"}>
      <span>{label}</span>
      <code>{value}</code>
    </div>
  );
}

function statusChipClass(audit: CodexSessionAudit | null, needsRepair: boolean) {
  if (!audit) return "sb-status-chip";
  if (needsRepair) return "sb-status-chip sb-status-chip-warn";
  return "sb-status-chip sb-status-chip-on";
}
