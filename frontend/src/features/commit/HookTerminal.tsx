import { useEffect, useMemo, useRef } from "react";
import type { GitHook, HookEvent } from "../../types/hooks";
import type { HookTerminalState } from "./useCommit";

interface HookTerminalProps {
  state: HookTerminalState;
  onClose: () => void;
}

const hookLabels: Record<GitHook, string> = {
  preCommit: "pre-commit",
  prepareCommitMsg: "prepare-commit-msg",
  commitMsg: "commit-msg",
  postCommit: "post-commit",
};

function eventText(event: HookEvent): string {
  const name = hookLabels[event.hook];
  switch (event.kind) {
    case "started":
      return `$ ${name}\n`;
    case "succeeded":
      return `\n[${name} completed successfully]\n`;
    case "failed":
      return `\n[${name} failed${event.exitCode === null ? "" : ` with exit code ${String(event.exitCode)}`}]\n`;
    case "stdout":
    case "stderr":
      return event.text ?? "";
  }
}

export function HookTerminal({ state, onClose }: HookTerminalProps) {
  const output = useRef<HTMLDivElement>(null);
  const activeHook = [...state.events]
    .reverse()
    .find((event) => event.kind === "started")?.hook;
  const failed = state.events.some((event) => event.kind === "failed");
  const status = state.running
    ? `Running ${activeHook === undefined ? "Git hook" : hookLabels[activeHook]}`
    : failed
      ? "Hook finished with errors"
      : "Hooks finished";
  const renderedEvents = useMemo(
    () =>
      state.events.map((event, index) => ({
        event,
        key: `${event.hook}-${event.kind}-${String(index)}`,
        text: eventText(event),
      })),
    [state.events],
  );

  useEffect(() => {
    const terminal = output.current;
    if (terminal !== null) {
      terminal.scrollTop = terminal.scrollHeight;
      if (state.open && state.events.length === 1) {
        terminal.focus();
      }
    }
  }, [state.events, state.open]);

  if (!state.open) {
    return null;
  }

  return (
    <div className="hook-terminal-backdrop" role="presentation">
      <section
        aria-labelledby="hook-terminal-heading"
        aria-modal="true"
        className="hook-terminal"
        role="dialog"
        onKeyDown={(event) => {
          event.stopPropagation();
          if (event.key === "Escape" && !state.running) {
            onClose();
          }
        }}
      >
        <header>
          <div>
            <span
              className={`hook-terminal-indicator ${state.running ? "running" : failed ? "failed" : ""}`}
            />
            <div>
              <h2 id="hook-terminal-heading">Git hooks</h2>
              <span aria-live="polite" role="status">{status}</span>
            </div>
          </div>
          <button
            aria-label="Close Git hook terminal"
            disabled={state.running}
            type="button"
            onClick={onClose}
          >
            ×
          </button>
        </header>
        <div
          ref={output}
          aria-live="off"
          className="hook-terminal-output"
          role="log"
          tabIndex={0}
        >
          {renderedEvents.map(({ event, key, text }) => (
            <span className={`hook-output-${event.kind}`} key={key}>
              {text}
            </span>
          ))}
          {state.running && <span className="hook-terminal-cursor" aria-hidden="true" />}
        </div>
        <footer>
          <span>Repository hooks run with the repository as their working directory.</span>
          <button disabled={state.running} type="button" onClick={onClose}>
            {state.running ? "Running…" : "Close"}
          </button>
        </footer>
      </section>
    </div>
  );
}
