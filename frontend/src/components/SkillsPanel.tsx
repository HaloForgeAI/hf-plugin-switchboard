import { ChevronDown, Copy, Download, Sparkles } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { SkillImportPatch } from "../types";

interface SkillsPanelProps {
  skillImportUrl: string;
  setSkillImportUrl: (value: string) => void;
  skillImport: SkillImportPatch | null;
  onImport: () => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function SkillsPanel({
  skillImportUrl,
  setSkillImportUrl,
  skillImport,
  onImport,
  t,
}: SkillsPanelProps) {
  const copyRepo = async () => {
    if (!skillImport?.repo) return;
    await navigator.clipboard.writeText(skillImport.repo);
  };

  return (
    <section className="sb-panel">
      <div className="sb-section-heading">
        <div>
          <h2>{t("switchboard.skills.title")}</h2>
          <p>{t("switchboard.skills.subtitle")}</p>
        </div>
        <span className="sb-tab-icon-badge">
          <Sparkles size={16} />
        </span>
      </div>

      <p className="sb-body-copy">{t("switchboard.skills.description")}</p>

      <details className="sb-details" open>
        <summary>
          <div className="sb-details-copy">
            <strong>{t("switchboard.skills.importTitle")}</strong>
            <span>{t("switchboard.skills.importHint")}</span>
          </div>
          <ChevronDown size={16} />
        </summary>
        <div className="sb-details-body">
          <div className="sb-ccswitch-row">
            <input
              value={skillImportUrl}
              onChange={(event) => setSkillImportUrl(event.target.value)}
              placeholder={t("switchboard.skills.importPlaceholder")}
              spellCheck={false}
            />
            <button type="button" className="sb-secondary-button" onClick={onImport}>
              <Download size={15} />
              {t("switchboard.skills.importAction")}
            </button>
          </div>
        </div>
      </details>

      {skillImport ? (
        <div className="sb-skill-preview">
          <div className="sb-skill-grid">
            <div>
              <span>{t("switchboard.skills.name")}</span>
              <strong>{skillImport.name}</strong>
            </div>
            <div>
              <span>{t("switchboard.skills.app")}</span>
              <strong>{skillImport.app}</strong>
            </div>
            <div>
              <span>{t("switchboard.skills.branch")}</span>
              <strong>{skillImport.branch || "main"}</strong>
            </div>
            <div>
              <span>{t("switchboard.skills.directory")}</span>
              <strong>{skillImport.directory || "/"}</strong>
            </div>
          </div>

          <label className="sb-block-field">
            <span>{t("switchboard.skills.repo")}</span>
            <div className="sb-copy-row">
              <input value={skillImport.repo} readOnly />
              <button type="button" className="sb-secondary-button" onClick={() => void copyRepo()}>
                <Copy size={15} />
                {t("switchboard.skills.copyRepo")}
              </button>
            </div>
          </label>

          <div className="sb-inline-card">
            <div className="sb-inline-card-head">
              <strong>{t("switchboard.skills.nextStep")}</strong>
            </div>
            <p className="sb-body-copy">{t("switchboard.skills.nextStepBody")}</p>
          </div>
        </div>
      ) : (
        <div className="sb-empty">{t("switchboard.skills.empty")}</div>
      )}
    </section>
  );
}
