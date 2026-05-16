import { Bot, Braces, Copy, TerminalSquare } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { TargetStatus } from "../types";

interface TargetCardProps {
  target: TargetStatus;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function TargetCard({ target, t }: TargetCardProps) {
  const Icon = target.id === "claude" ? Bot : target.id === "codex" ? Braces : TerminalSquare;

  return (
    <article className="sb-target-card">
      <div className="sb-target-head">
        <div className="sb-target-title-row">
          <span className="sb-target-icon">
            <Icon size={16} />
          </span>
          <div>
            <h2>{target.label}</h2>
          </div>
        </div>
        <span className={target.configured ? "sb-status-chip sb-status-chip-on" : "sb-status-chip"}>
          {target.configured ? t("switchboard.target.configured") : t("switchboard.target.empty")}
        </span>
      </div>
      <div className="sb-path-list">
        {target.details.length > 0 && (
          <div className="sb-target-detail-grid">
            {target.details.map((detail) => (
              <DetailRow key={`${target.id}:${detail.label}`} label={detail.label} value={detail.value} secret={detail.secret} />
            ))}
          </div>
        )}
        {target.paths.map((path) => (
          <div key={`${target.id}:${path.label}`} className="sb-path-row">
            <div className="sb-path-copy">
              <span>{path.label}</span>
              <code>{path.path}</code>
            </div>
            <span className={path.exists ? "sb-path-pill sb-path-pill-on" : "sb-path-pill"}>
              {path.exists ? t("switchboard.target.exists") : t("switchboard.target.missing")}
            </span>
          </div>
        ))}
      </div>
    </article>
  );
}

function DetailRow({ label, value, secret }: { label: string; value: string; secret?: string }) {
  const copyValue = secret || value;
  return (
    <div className="sb-target-detail">
      <span>{label}</span>
      <div className={secret ? "sb-secret-copy" : "sb-detail-copy"}>
        {secret ? (
          <>
            <code className="sb-secret-mask">{value}</code>
            <code className="sb-secret-reveal">{secret}</code>
          </>
        ) : (
          <code>{value}</code>
        )}
        <button
          type="button"
          className="sb-mini-icon-button"
          onClick={() => void navigator.clipboard?.writeText(copyValue)}
          title="Copy"
        >
          <Copy size={12} />
        </button>
      </div>
    </div>
  );
}
