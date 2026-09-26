import { useEffect, useRef, useState } from "react";
import { Icon } from "../../components/Icon";
import type { RemoteService } from "../../services/remoteService";
import type { RepositorySnapshot } from "../../types/repository";
import type { RemoteOperation, RemoteProgress } from "../../types/remotes";
import { useRemotes } from "./useRemotes";

interface RemoteControlsProps {
  repository: RepositorySnapshot | null;
  repositoryRevision: number;
  service: RemoteService;
  onUpdated: (snapshot: RepositorySnapshot) => void;
}

function operationLabel(
  operation: RemoteOperation,
  remoteName: string,
  ahead: number,
): string {
  if (operation === "push" && ahead > 0) {
    return `Pushing ${String(ahead)} commit${ahead === 1 ? "" : "s"}…`;
  }
  const verb = operation === "fetch" ? "Fetching" : operation === "pull" ? "Pulling" : "Pushing";
  return `${verb} ${remoteName}…`;
}

function progressDetail(progress: RemoteProgress): string {
  switch (progress.phase) {
    case "connecting":
      return "Connecting securely…";
    case "transferring":
      if (progress.total > 0) {
        return `Transferring objects ${String(progress.current)} of ${String(progress.total)}`;
      }
      return progress.bytes > 0
        ? `Transferring objects · ${String(Math.max(1, Math.round(progress.bytes / 1024)))} KB`
        : "Transferring objects…";
    case "updating":
      return "Updating remote-tracking references…";
    case "merging":
      return "Integrating remote changes…";
    case "completed":
      return "Finishing…";
  }
}

export function RemoteControls({
  repository,
  repositoryRevision,
  service,
  onUpdated,
}: RemoteControlsProps) {
  const [open, setOpen] = useState(false);
  const [lastOperation, setLastOperation] = useState<RemoteOperation>("fetch");
  const root = useRef<HTMLDivElement>(null);
  const state = useRemotes(
    service,
    repository?.path ?? null,
    repositoryRevision,
    onUpdated,
  );
  const usable = state.remote !== null && state.remote.kind !== "unsupported";
  const remoteName = state.remote?.name ?? "remote";

  useEffect(() => {
    setOpen(false);
    setLastOperation("fetch");
  }, [repository?.id]);

  useEffect(() => {
    if (state.busy !== null || state.error !== null) {
      setOpen(true);
    }
  }, [state.busy, state.error]);

  useEffect(() => {
    if (!open) {
      return undefined;
    }
    const closeOnPointer = (event: PointerEvent) => {
      if (root.current?.contains(event.target as Node) === false) {
        setOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && state.busy === null) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOnPointer);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnPointer);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open, state.busy]);

  const run = (operation: RemoteOperation) => {
    setLastOperation(operation);
    setOpen(true);
    if (operation === "fetch") {
      void state.fetch();
    } else if (operation === "pull") {
      void state.pull();
    } else {
      void state.push();
    }
  };

  const activeLabel =
    state.busy === null
      ? `Fetch ${remoteName}`
      : operationLabel(state.busy, remoteName, state.remote?.ahead ?? 0);

  return (
    <div className="remote-control" ref={root}>
      <div className="remote-button-group">
        <button
          className="remote-fetch-button"
          type="button"
          disabled={repository === null || state.loading || state.busy !== null || !usable}
          onClick={() => run("fetch")}
        >
          {state.busy !== null ? <span className="loading-spinner" /> : <Icon name="refresh" />}
          <span>{state.loading ? "Checking…" : activeLabel}</span>
        </button>
        <button
          className="remote-menu-button"
          type="button"
          aria-label="Remote actions"
          aria-haspopup="dialog"
          aria-expanded={open}
          disabled={repository === null}
          onClick={() => setOpen((current) => !current)}
        >
          <Icon name="arrow" />
        </button>
      </div>

      {open && repository !== null && (
        <div className="remote-popover" role="dialog" aria-label="Remote actions">
          <div className="remote-popover-header">
            <div>
              <strong>{state.remote?.name ?? "No remote"}</strong>
              <span>
                {state.remote === null
                  ? "This repository has no configured remote."
                  : state.remote.displayUrl}
              </span>
            </div>
            {state.remote !== null && (
              <div className="remote-counts" aria-label={`${String(state.remote.ahead)} ahead, ${String(state.remote.behind)} behind`}>
                <span>↑ {state.remote.ahead}</span>
                <span>↓ {state.remote.behind}</span>
              </div>
            )}
          </div>

          {state.remote?.kind === "unsupported" && (
            <div className="remote-guidance" role="status">
              <strong>Unsupported remote address</strong>
              <span>Branchcraft supports local remotes and credential-free GitHub HTTPS URLs. Change this remote to HTTPS before continuing.</span>
            </div>
          )}

          {state.busy !== null && state.progress !== null && (
            <div className="remote-progress" role="status" aria-live="polite">
              <strong>{operationLabel(state.busy, remoteName, state.remote?.ahead ?? 0)}</strong>
              <span>{progressDetail(state.progress)}</span>
              <progress
                max={Math.max(1, state.progress.total)}
                value={state.progress.total > 0 ? state.progress.current : undefined}
              />
            </div>
          )}

          {state.error !== null && (
            <div className="remote-error" role="alert">
              <strong>{state.error.summary}</strong>
              {state.error.detail !== null && <span>{state.error.detail}</span>}
              <div>
                <button type="button" disabled={state.busy !== null} onClick={() => run(lastOperation)}>
                  Retry
                </button>
                <button type="button" onClick={state.dismissError}>Dismiss</button>
              </div>
            </div>
          )}

          {state.notice !== null && (
            <div className="remote-notice" role="status">
              <span>✓</span>
              <strong>{state.notice}</strong>
              <button type="button" aria-label="Dismiss remote status" onClick={state.dismissNotice}>×</button>
            </div>
          )}

          <div className="remote-actions">
            <button
              type="button"
              disabled={!usable || state.busy !== null}
              onClick={() => run("fetch")}
            >
              <strong>Fetch</strong>
              <span>Download remote updates</span>
            </button>
            <button
              type="button"
              disabled={!usable || state.busy !== null}
              onClick={() => run("pull")}
            >
              <strong>Pull</strong>
              <span>Fetch and integrate changes</span>
            </button>
            <button
              type="button"
              disabled={!usable || state.busy !== null}
              onClick={() => run("push")}
            >
              <strong>Push{state.remote !== null && state.remote.ahead > 0 ? ` ${String(state.remote.ahead)}` : ""}</strong>
              <span>Publish local commits</span>
            </button>
          </div>

          {state.remote !== null && !state.remote.tracking && (
            <p className="remote-footnote">The current branch has no explicit upstream; Branchcraft will use {state.remote.name} and the matching branch name.</p>
          )}
        </div>
      )}
    </div>
  );
}
