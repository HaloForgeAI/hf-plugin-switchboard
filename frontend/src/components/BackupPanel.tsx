import { FileText, RotateCcw, Shield } from "lucide-react";
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
              <div className="sb-backup-main">
                <strong>{formatBackupTime(backup.createdAt, backup.id)}</strong>
                <span>{t("switchboard.backups.files", { count: backup.files.length })}</span>
                <div className="sb-backup-files">
                  {backup.files.map((file) => (
                    <details key={`${backup.id}:${file.originalPath}`} className="sb-backup-file">
                      <summary>
                        <FileText size={12} />
                        <span>{fileLabel(file.originalPath)}</span>
                        <small>
                          {file.existed
                            ? t("switchboard.backups.savedBytes", { count: file.byteCount ?? 0 })
                            : t("switchboard.backups.createdByChange")}
                        </small>
                      </summary>
                      <code>{file.originalPath}</code>
                      {file.preview && <pre>{file.preview}</pre>}
                    </details>
                  ))}
                </div>
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

function formatBackupTime(createdAt: string, fallback: string) {
  const numeric = Number(createdAt);
  if (Number.isFinite(numeric) && numeric > 0) {
    const millis = createdAt.length > 11 ? Math.floor(numeric / 1_000_000) : numeric * 1000;
    const date = new Date(millis);
    if (!Number.isNaN(date.getTime())) {
      return date.toLocaleString();
    }
  }
  return fallback;
}

function fileLabel(path: string) {
  return path.split(/[\\/]/).filter(Boolean).pop() || path;
}
