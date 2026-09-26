import type { HistoryService } from "../../services/historyService";
import { useHistory } from "./useHistory";

interface HistoryPanelProps {
  repositoryPath: string;
  repositoryRevision: number;
  service: HistoryService;
}

function formatTime(seconds: number): string {
  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(seconds * 1000));
}

export function HistoryPanel({
  repositoryPath,
  repositoryRevision,
  service,
}: HistoryPanelProps) {
  const state = useHistory(service, repositoryPath, repositoryRevision);

  if (state.loading) {
    return (
      <section className="history-panel" aria-label="Commit history">
        <div className="history-state" role="status">
          <span className="loading-spinner" />
          Loading history…
        </div>
      </section>
    );
  }

  if (state.error !== null) {
    return (
      <section className="history-panel" aria-label="Commit history">
        <div className="history-state" role="alert">
          <strong>{state.error.summary}</strong>
          {state.error.detail !== null && <span>{state.error.detail}</span>}
          <button type="button" onClick={state.reload}>
            Try again
          </button>
        </div>
      </section>
    );
  }

  if (state.history === null || state.history.commits.length === 0) {
    return (
      <section className="history-panel" aria-label="Commit history">
        <div className="history-state">
          <strong>No commits yet</strong>
          <span>Create the first commit to begin this repository’s history.</span>
        </div>
      </section>
    );
  }

  return (
    <section className="history-panel" aria-label="Commit history">
      <ol className="history-list">
        {state.history.commits.map((commit) => (
          <li key={commit.id}>
            <div className="history-marker" />
            <div className="history-copy">
              <strong>{commit.summary}</strong>
              {commit.description !== null && <p>{commit.description}</p>}
              <span title={commit.authorEmail}>
                {commit.authorName} · {formatTime(commit.authoredAt)}
              </span>
            </div>
            <code>{commit.shortId}</code>
          </li>
        ))}
      </ol>
      {state.history.truncated && (
        <p className="history-limit">Showing the newest 200 commits.</p>
      )}
    </section>
  );
}
