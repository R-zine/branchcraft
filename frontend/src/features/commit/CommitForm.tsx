import type { CommitService } from "../../services/commitService";
import type { RepositorySnapshot } from "../../types/repository";
import { HookTerminal } from "./HookTerminal";
import { useCommit } from "./useCommit";

interface CommitFormProps {
  repository: RepositorySnapshot;
  service: CommitService;
  disabled: boolean;
  onCommitted: (snapshot: RepositorySnapshot) => void;
}

export function CommitForm({
  repository,
  service,
  disabled,
  onCommitted,
}: CommitFormProps) {
  const stagedCount = repository.files.filter(
    (file) => file.staged !== null,
  ).length;
  const state = useCommit(service, repository.path, stagedCount, onCommitted);
  const branch = repository.branch.detached
    ? "detached HEAD"
    : repository.branch.displayName;

  return (
    <>
      <form
        className="commit-form"
        aria-label="Create commit"
        onSubmit={(event) => {
          event.preventDefault();
          void state.submit();
        }}
      >
      <div className="commit-heading">
        <strong>Commit</strong>
        <span>{stagedCount} staged</span>
      </div>
      <label>
        <span className="visually-hidden">Summary</span>
        <input
          type="text"
          value={state.summary}
          maxLength={120}
          placeholder="Summary (required)"
          disabled={state.committing || disabled}
          onChange={(event) => state.setSummary(event.target.value)}
        />
      </label>
      <label>
        <span className="visually-hidden">Description</span>
        <textarea
          value={state.description}
          maxLength={10000}
          placeholder="Description"
          disabled={state.committing || disabled}
          onChange={(event) => state.setDescription(event.target.value)}
        />
      </label>

      {state.error !== null && (
        <div className="commit-message error" role="alert">
          <strong>{state.error.summary}</strong>
          {state.error.detail !== null && <span>{state.error.detail}</span>}
        </div>
      )}
      {state.lastCommit !== null && (
        <div className="commit-message success" role="status">
          <strong>Committed {state.lastCommit.shortId}</strong>
          <span>{state.lastCommit.summary}</span>
        </div>
      )}
      {state.error === null && state.lastCommit === null && stagedCount === 0 && (
        <p className="commit-hint">Stage at least one file to create a commit.</p>
      )}

      <button
        className="commit-button"
        type="submit"
        disabled={state.committing || disabled}
      >
        {state.committing ? "Committing…" : `Commit to ${branch}`}
      </button>
      </form>
      <HookTerminal state={state.hookTerminal} onClose={state.closeHookTerminal} />
    </>
  );
}
