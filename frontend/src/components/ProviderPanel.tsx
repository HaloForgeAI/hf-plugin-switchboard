import { AppSelect } from "@haloforge/plugin-sdk";
import { CheckCircle2, ChevronDown, Download, Settings2 } from "lucide-react";
import type { SwitchboardTranslationKey } from "../i18n";
import type { ProviderForm, SetProviderForm, TargetStatus } from "../types";

interface ProviderPanelProps {
  target: "claude" | "codex";
  status?: TargetStatus;
  form: ProviderForm;
  setForm: SetProviderForm;
  ccswitchUrl: string;
  setCcswitchUrl: (value: string) => void;
  busy: string | null;
  onImport: () => void;
  onApply: () => void;
  t: (key: SwitchboardTranslationKey, vars?: Record<string, string | number>) => string;
}

export function ProviderPanel({
  target,
  status,
  form,
  setForm,
  ccswitchUrl,
  setCcswitchUrl,
  busy,
  onImport,
  onApply,
  t,
}: ProviderPanelProps) {
  const isClaude = target === "claude";

  return (
    <section className="sb-panel">
      <div className="sb-section-heading">
        <div>
          <h2>{isClaude ? t("switchboard.provider.claudeTitle") : t("switchboard.provider.codexTitle")}</h2>
          <p>{isClaude ? t("switchboard.provider.claudeSubtitle") : t("switchboard.provider.codexSubtitle")}</p>
        </div>
      </div>

      {status && (
        <div className="sb-inline-card">
          <div className="sb-inline-card-head">
            <strong>{isClaude ? t("switchboard.provider.claudePathTitle") : t("switchboard.provider.codexPathTitle")}</strong>
            <span className={status.configured ? "sb-status-chip sb-status-chip-on" : "sb-status-chip"}>
              {status.configured ? t("switchboard.target.configured") : t("switchboard.target.empty")}
            </span>
          </div>
          <div className="sb-inline-paths">
            {status.paths.map((path) => (
              <div className="sb-inline-path" key={`${status.id}:${path.label}`}>
                <span>{path.label}</span>
                <code>{path.path}</code>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="sb-form-grid">
        <label>
          <span>{t("switchboard.provider.baseUrl")}</span>
          <input
            value={form.baseUrl}
            onChange={(event) => updateForm(setForm, { baseUrl: event.target.value })}
            placeholder="https://api.example.com"
            spellCheck={false}
          />
        </label>
        <label>
          <span>{t("switchboard.provider.apiKey")}</span>
          <input
            value={form.apiKey}
            onChange={(event) => updateForm(setForm, { apiKey: event.target.value })}
            type="password"
            spellCheck={false}
          />
        </label>
        <label>
          <span>{t("switchboard.provider.model")}</span>
          <input
            value={form.model}
            onChange={(event) => updateForm(setForm, { model: event.target.value })}
            spellCheck={false}
          />
        </label>
      </div>

      <details className="sb-details">
        <summary>
          <div className="sb-details-copy">
            <strong>{t("switchboard.provider.importTitle")}</strong>
            <span>{t("switchboard.provider.importHint")}</span>
          </div>
          <ChevronDown size={16} />
        </summary>
        <div className="sb-details-body">
          <div className="sb-ccswitch-row">
            <input
              value={ccswitchUrl}
              onChange={(event) => setCcswitchUrl(event.target.value)}
              placeholder={t("switchboard.provider.importPlaceholder")}
              spellCheck={false}
            />
            <button type="button" className="sb-secondary-button" onClick={onImport}>
              <Download size={15} />
              {t("switchboard.provider.importAction")}
            </button>
          </div>
        </div>
      </details>

      <details className="sb-details">
        <summary>
          <div className="sb-details-copy">
            <strong>{t("switchboard.provider.advancedTitle")}</strong>
            <span>{t("switchboard.common.collapsed")}</span>
          </div>
          <ChevronDown size={16} />
        </summary>
        <div className="sb-details-body">
          {isClaude ? (
            <>
              <div className="sb-form-grid">
                <label>
                  <span>{t("switchboard.provider.modelHaiku")}</span>
                  <input
                    value={form.haikuModel}
                    onChange={(event) => updateForm(setForm, { haikuModel: event.target.value })}
                    spellCheck={false}
                  />
                </label>
                <label>
                  <span>{t("switchboard.provider.modelSonnet")}</span>
                  <input
                    value={form.sonnetModel}
                    onChange={(event) => updateForm(setForm, { sonnetModel: event.target.value })}
                    spellCheck={false}
                  />
                </label>
                <label>
                  <span>{t("switchboard.provider.modelOpus")}</span>
                  <input
                    value={form.opusModel}
                    onChange={(event) => updateForm(setForm, { opusModel: event.target.value })}
                    spellCheck={false}
                  />
                </label>
              </div>
              <div className="sb-options">
                <label>
                  <input
                    type="checkbox"
                    checked={form.setClaudePrimaryApiKey}
                    onChange={(event) => updateForm(setForm, { setClaudePrimaryApiKey: event.target.checked })}
                  />
                  <span>{t("switchboard.provider.primaryApiKey")}</span>
                </label>
                <label>
                  <input
                    type="checkbox"
                    checked={form.skipClaudeOnboarding}
                    onChange={(event) => updateForm(setForm, { skipClaudeOnboarding: event.target.checked })}
                  />
                  <span>{t("switchboard.provider.skipOnboarding")}</span>
                </label>
              </div>
            </>
          ) : (
            <div className="sb-form-grid">
              <label>
                <span>{t("switchboard.provider.providerId")}</span>
                <input
                  value={form.providerId}
                  onChange={(event) => updateForm(setForm, { providerId: event.target.value })}
                  spellCheck={false}
                />
              </label>
              <label>
                <span>{t("switchboard.provider.reasoning")}</span>
                <AppSelect
                  value={form.reasoningEffort}
                  onChange={(event) => updateForm(setForm, { reasoningEffort: event.target.value })}
                >
                  <option value="high">{t("switchboard.provider.reasoning.high")}</option>
                  <option value="medium">{t("switchboard.provider.reasoning.medium")}</option>
                  <option value="low">{t("switchboard.provider.reasoning.low")}</option>
                </AppSelect>
              </label>
            </div>
          )}
        </div>
      </details>

      <div className="sb-actions">
        <button type="button" className="sb-primary-button" disabled={busy === "provider"} onClick={onApply}>
          {isClaude ? <Settings2 size={16} /> : <CheckCircle2 size={16} />}
          {isClaude ? t("switchboard.provider.applyClaude") : t("switchboard.provider.applyCodex")}
        </button>
      </div>
    </section>
  );
}

function updateForm(setForm: SetProviderForm, patch: Partial<ProviderForm>) {
  setForm((current) => ({ ...current, ...patch }));
}
