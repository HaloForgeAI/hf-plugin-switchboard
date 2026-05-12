import { RotateCcw, Shield } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { BackupInfo } from "../types";

interface BackupPanelProps {
  backups: BackupInfo[];
  busy: string | null;
  onRestore: (backupId: string) => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function BackupPanel({ backups, busy, onRestore, t }: BackupPanelProps) {
  return (
    <section className="sb-panel">
      <div className="sb-section-heading">
        <div>
          <h2>{t("switchboard.backups.title")}</h2>
          <p>{t("switchboard.backups.subtitle")}</p>
        </div>
        <span className="sb-tab-icon-badge">
          <Shield size={16} />
        </span>
      </div>
      <div className="sb-backup-list">
        {backups.length ? (
          backups.slice(0, 12).map((backup) => (
            <div className="sb-backup-item" key={backup.id}>
              <div>
                <strong>{backup.id}</strong>
                <span>{t("switchboard.backups.files", { count: backup.files.length })}</span>
              </div>
              <button
                type="button"
                className="sb-icon-button"
                disabled={busy === `restore:${backup.id}`}
                onClick={() => onRestore(backup.id)}
                title={t("switchboard.backups.restore")}
              >
                <RotateCcw size={16} />
              </button>
            </div>
          ))
        ) : (
          <div className="sb-empty">{t("switchboard.backups.empty")}</div>
        )}
      </div>
    </section>
  );
}
