import { RotateCcw, Shield } from "lucide-react";
import type { BackupInfo } from "../types";

interface BackupPanelProps {
  backups: BackupInfo[];
  busy: string | null;
  onRestore: (backupId: string) => void;
}

export function BackupPanel({ backups, busy, onRestore }: BackupPanelProps) {
  return (
    <div className="sb-panel">
      <div className="sb-panel-title">
        <Shield size={18} />
        <h2>Backups</h2>
      </div>
      <div className="sb-backup-list">
        {backups.length ? (
          backups.slice(0, 8).map((backup) => (
            <div className="sb-backup-item" key={backup.id}>
              <div>
                <strong>{backup.id}</strong>
                <span>{backup.files.length} file(s)</span>
              </div>
              <button
                type="button"
                className="sb-icon-button"
                disabled={busy === `restore:${backup.id}`}
                onClick={() => onRestore(backup.id)}
                title="Restore"
              >
                <RotateCcw size={16} />
              </button>
            </div>
          ))
        ) : (
          <div className="sb-empty">No backups</div>
        )}
      </div>
    </div>
  );
}
