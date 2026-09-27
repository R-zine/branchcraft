import { Icon } from "../../components/Icon";
import type { FileStatus, GitError } from "../../types/repository";
import { presentGitPath } from "../../utils/gitPath";
import { statusCode, statusLabel, statusTone } from "./statusPresentation";

interface ChangesPanelProps {
  files: FileStatus[];
  selectedPath: string | null;
  busy: boolean;
  pendingPath: string | null;
  error: GitError | null;
  onSelect: (path: string) => void;
  onStageFile: (path: string) => void;
  onUnstageFile: (path: string) => void;
  onStageAll: () => void;
  onUnstageAll: () => void;
  onDismissError: () => void;
}

export function ChangesPanel({
  files,
  selectedPath,
  busy,
  pendingPath,
  error,
  onSelect,
  onStageFile,
  onUnstageFile,
  onStageAll,
  onUnstageAll,
  onDismissError,
}: ChangesPanelProps) {
  const hasStaged = files.some(
    (file) => file.staged !== null || file.conflicted,
  );
  const hasUnstaged = files.some(
    (file) => file.unstaged !== null || file.conflicted,
  );

  return (
    <aside className="changes-panel" aria-label="Working tree changes">
      <div className="panel-heading">
        <div>
          <div>
            <h2>Changes</h2>
            <span>
              {files.length} {files.length === 1 ? "file" : "files"}
            </span>
          </div>
          <div className="bulk-actions">
            <button
              type="button"
              disabled={busy || !hasUnstaged}
              onClick={onStageAll}
            >
              Stage all
            </button>
            <button
              type="button"
              disabled={busy || !hasStaged}
              onClick={onUnstageAll}
            >
              Unstage all
            </button>
          </div>
        </div>
      </div>

      {error !== null && (
        <div className="staging-error" role="alert">
          <div>
            <strong>{error.summary}</strong>
            {error.detail !== null && <span>{error.detail}</span>}
          </div>
          <button
            type="button"
            aria-label="Dismiss staging error"
            onClick={onDismissError}
          >
            ×
          </button>
        </div>
      )}

      {files.length === 0 ? (
        <div className="clean-state">
          <span className="clean-check">✓</span>
          <strong>No local changes</strong>
          <span>Your working tree is clean.</span>
        </div>
      ) : (
        <ul className="file-list">
          {files.map((file) => {
            const shouldStage = file.unstaged !== null || file.conflicted;
            const path = presentGitPath(file.path);
            const stageState =
              file.staged !== null && file.unstaged === null && !file.conflicted
                ? "true"
                : file.staged !== null || file.conflicted
                  ? "mixed"
                  : "false";
            return (
              <li
                key={file.path}
                className={file.path === selectedPath ? "selected" : undefined}
              >
                <button
                  className="stage-toggle"
                  type="button"
                  role="checkbox"
                  aria-checked={stageState}
                  aria-label={`${shouldStage ? "Stage" : "Unstage"} ${file.path}`}
                  disabled={busy}
                  onClick={() =>
                    shouldStage
                      ? onStageFile(file.path)
                      : onUnstageFile(file.path)
                  }
                >
                  <span>
                    {stageState === "true"
                      ? "✓"
                      : stageState === "mixed"
                        ? "−"
                        : ""}
                  </span>
                </button>
                <button
                  className="file-select"
                  type="button"
                  onClick={() => onSelect(file.path)}
                >
                  <span className={`status-code ${statusTone(file)}`}>
                    {pendingPath === file.path ? "…" : statusCode(file)}
                  </span>
                  <Icon name="file" />
                  <span className="file-copy">
                    <strong>{path.filename}</strong>
                    <span>{path.parent}</span>
                  </span>
                  <span className="visually-hidden">{statusLabel(file)}</span>
                </button>
              </li>
            );
          })}
        </ul>
      )}
    </aside>
  );
}

