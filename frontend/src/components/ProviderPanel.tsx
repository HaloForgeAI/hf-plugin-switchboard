import { AppSelect } from "@haloforge/plugin-sdk";
import { CheckCircle2, Download, PlugZap } from "lucide-react";
import type { SetProviderForm, Target, ProviderForm } from "../types";

interface ProviderPanelProps {
  form: ProviderForm;
  setForm: SetProviderForm;
  ccswitchUrl: string;
  setCcswitchUrl: (value: string) => void;
  busy: string | null;
  onImport: () => void;
  onApply: () => void;
}

export function ProviderPanel({
  form,
  setForm,
  ccswitchUrl,
  setCcswitchUrl,
  busy,
  onImport,
  onApply,
}: ProviderPanelProps) {
  return (
    <div className="sb-panel sb-provider-panel">
      <div className="sb-panel-title">
        <PlugZap size={18} />
        <h2>Provider</h2>
      </div>

      <div className="sb-ccswitch-row">
        <input
          value={ccswitchUrl}
          onChange={(event) => setCcswitchUrl(event.target.value)}
          placeholder="ccswitch://v1/import?resource=provider..."
          spellCheck={false}
        />
        <button type="button" className="sb-secondary-button" onClick={onImport}>
          <Download size={15} />
          Import
        </button>
      </div>

      <div className="sb-form-grid">
        <label>
          <span>Target</span>
          <AppSelect value={form.target} onChange={(event) => updateForm(setForm, { target: event.target.value as Target })}>
            <option value="both">Claude + Codex</option>
            <option value="claude">Claude Code</option>
            <option value="codex">Codex</option>
          </AppSelect>
        </label>
        <label>
          <span>Name</span>
          <input value={form.name} onChange={(event) => updateForm(setForm, { name: event.target.value })} />
        </label>
        <label>
          <span>Base URL</span>
          <input
            value={form.baseUrl}
            onChange={(event) => updateForm(setForm, { baseUrl: event.target.value })}
            placeholder="https://api.example.com"
            spellCheck={false}
          />
        </label>
        <label>
          <span>API key</span>
          <input
            value={form.apiKey}
            onChange={(event) => updateForm(setForm, { apiKey: event.target.value })}
            type="password"
            spellCheck={false}
          />
        </label>
        <label>
          <span>Model</span>
          <input
            value={form.model}
            onChange={(event) => updateForm(setForm, { model: event.target.value })}
            placeholder="auto"
            spellCheck={false}
          />
        </label>
        <label>
          <span>Codex provider id</span>
          <input
            value={form.providerId}
            onChange={(event) => updateForm(setForm, { providerId: event.target.value })}
            spellCheck={false}
          />
        </label>
      </div>

      <div className="sb-options">
        <label>
          <input
            type="checkbox"
            checked={form.skipClaudeOnboarding}
            onChange={(event) => updateForm(setForm, { skipClaudeOnboarding: event.target.checked })}
          />
          <span>Claude onboarding flag</span>
        </label>
        <label>
          <input
            type="checkbox"
            checked={form.setClaudePrimaryApiKey}
            onChange={(event) => updateForm(setForm, { setClaudePrimaryApiKey: event.target.checked })}
          />
          <span>Claude primaryApiKey</span>
        </label>
      </div>

      <div className="sb-actions">
        <button type="button" className="sb-primary-button" disabled={busy === "provider"} onClick={onApply}>
          <CheckCircle2 size={16} />
          Apply
        </button>
      </div>
    </div>
  );
}

function updateForm(setForm: SetProviderForm, patch: Partial<ProviderForm>) {
  setForm((current) => ({ ...current, ...patch }));
}
