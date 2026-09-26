import { useEffect, useRef } from "react";
import type { SettingsState } from "./useSettings";

interface SettingsPanelProps {
  state: SettingsState;
  onClose: () => void;
}

const shortcuts = [
  ["Open repository", "Ctrl+O"],
  ["Clone repository", "Ctrl+Shift+O"],
  ["Refresh repository", "Ctrl+R"],
  ["Show changes", "Ctrl+1"],
  ["Show history", "Ctrl+2"],
  ["Open settings", "Ctrl+,"],
] as const;

export function SettingsPanel({ state, onClose }: SettingsPanelProps) {
  const closeButton = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    closeButton.current?.focus();
  }, []);

  return (
    <div
      className="settings-backdrop"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) {
          onClose();
        }
      }}
    >
      <section
        aria-labelledby="settings-heading"
        aria-modal="true"
        className="settings-panel"
        role="dialog"
      >
        <header>
          <div>
            <span>Branchcraft</span>
            <h2 id="settings-heading">Settings</h2>
          </div>
          <button
            ref={closeButton}
            aria-label="Close settings"
            type="button"
            onClick={onClose}
          >
            ×
          </button>
        </header>

        {state.error !== null && (
          <div className="settings-error" role="alert">
            <div>
              <strong>{state.error.summary}</strong>
              {state.error.detail !== null && <span>{state.error.detail}</span>}
            </div>
            <div>
              {state.error.kind === "corrupt" && (
                <button type="button" disabled={state.busy} onClick={() => void state.reset()}>
                  Reset settings
                </button>
              )}
              <button type="button" onClick={state.dismissError}>Dismiss</button>
            </div>
          </div>
        )}

        <div className="settings-content">
          <section aria-labelledby="appearance-heading">
            <div className="settings-section-heading">
              <h3 id="appearance-heading">Appearance</h3>
              <span>Match your desktop or choose a fixed theme.</span>
            </div>
            <fieldset className="theme-options" disabled={state.loading || state.busy}>
              <legend className="visually-hidden">Application theme</legend>
              {(["system", "light", "dark"] as const).map((theme) => (
                <label key={theme}>
                  <input
                    checked={state.settings.theme === theme}
                    name="theme"
                    type="radio"
                    value={theme}
                    onChange={() => void state.setTheme(theme)}
                  />
                  <span>{theme.charAt(0).toUpperCase() + theme.slice(1)}</span>
                </label>
              ))}
            </fieldset>
          </section>

          <section aria-labelledby="recent-heading">
            <div className="settings-section-heading settings-recent-heading">
              <div>
                <h3 id="recent-heading">Recent repositories</h3>
                <span>Stored locally in Branchcraft settings.</span>
              </div>
              {state.settings.recentRepositories.length > 0 && (
                <button
                  type="button"
                  disabled={state.busy}
                  onClick={() => void state.clearRecentRepositories()}
                >
                  Clear all
                </button>
              )}
            </div>
            {state.settings.recentRepositories.length === 0 ? (
              <p className="settings-empty">No recently opened repositories.</p>
            ) : (
              <ul className="settings-recents">
                {state.settings.recentRepositories.map((repository) => (
                  <li key={repository.path}>
                    <div>
                      <strong>{repository.name}</strong>
                      <span>{repository.path}</span>
                    </div>
                    {!repository.available && <em>Unavailable</em>}
                    <button
                      aria-label={`Remove ${repository.name} from recent repositories`}
                      type="button"
                      disabled={state.busy}
                      onClick={() => void state.removeRecentRepository(repository.path)}
                    >
                      Remove
                    </button>
                  </li>
                ))}
              </ul>
            )}
          </section>

          <section aria-labelledby="shortcuts-heading">
            <div className="settings-section-heading">
              <h3 id="shortcuts-heading">Keyboard shortcuts</h3>
              <span>Use Command instead of Control on macOS.</span>
            </div>
            <dl className="shortcut-list">
              {shortcuts.map(([label, shortcut]) => (
                <div key={label}>
                  <dt>{label}</dt>
                  <dd><kbd>{shortcut}</kbd></dd>
                </div>
              ))}
            </dl>
          </section>

          <section className="about-branchcraft" aria-labelledby="about-heading">
            <div className="settings-section-heading">
              <h3 id="about-heading">About Branchcraft</h3>
              <span>Version 0.1.0 · MIT License</span>
            </div>
            <p>A fast, focused desktop Git client built with Rust, Tauri, and React.</p>
          </section>
        </div>
      </section>
    </div>
  );
}
