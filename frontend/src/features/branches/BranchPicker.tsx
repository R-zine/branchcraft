import { useEffect, useMemo, useRef, useState } from "react";
import { Icon } from "../../components/Icon";
import type { BranchService } from "../../services/branchService";
import type { BranchSummary } from "../../types/branches";
import type { RepositorySnapshot } from "../../types/repository";
import { useBranches } from "./useBranches";

interface BranchPickerProps {
  repository: RepositorySnapshot | null;
  service: BranchService;
  onUpdated: (snapshot: RepositorySnapshot) => void;
}

function BranchRow({
  branch,
  busy,
  onCheckout,
  onDelete,
}: {
  branch: BranchSummary;
  busy: boolean;
  onCheckout: (name: string) => void;
  onDelete: (name: string) => void;
}) {
  return (
    <li className={branch.current ? "current" : undefined}>
      <button
        className="branch-row-main"
        type="button"
        aria-label={
          branch.current
            ? `Current branch ${branch.name}`
            : branch.kind === "remote"
              ? `Remote-tracking branch ${branch.name}`
              : `Switch to branch ${branch.name}`
        }
        disabled={busy || branch.current || branch.kind === "remote"}
        onClick={() => onCheckout(branch.name)}
      >
        <Icon name="branch" />
        <span>
          <strong>{branch.name}</strong>
          <small>
            {branch.current
              ? "Current branch"
              : branch.kind === "remote"
                ? "Remote-tracking"
                : branch.upstream ?? branch.shortTarget}
          </small>
        </span>
      </button>
      {branch.kind === "local" && !branch.current && (
        <button
          className="branch-delete-button"
          type="button"
          aria-label={`Delete ${branch.name}`}
          disabled={busy}
          onClick={() => onDelete(branch.name)}
        >
          ×
        </button>
      )}
    </li>
  );
}

export function BranchPicker({
  repository,
  service,
  onUpdated,
}: BranchPickerProps) {
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const [creating, setCreating] = useState(false);
  const [newBranch, setNewBranch] = useState("");
  const [deleteCandidate, setDeleteCandidate] = useState<string | null>(null);
  const root = useRef<HTMLDivElement>(null);
  const state = useBranches(service, repository?.path ?? null, open, onUpdated);
  const normalizedQuery = query.trim().toLowerCase();
  const visibleBranches = useMemo(
    () =>
      state.branches.filter(
        (branch) =>
          normalizedQuery === "" || branch.name.toLowerCase().includes(normalizedQuery),
      ),
    [normalizedQuery, state.branches],
  );
  const localBranches = visibleBranches.filter((branch) => branch.kind === "local");
  const remoteBranches = visibleBranches.filter((branch) => branch.kind === "remote");

  useEffect(() => {
    setOpen(false);
    setQuery("");
    setCreating(false);
    setNewBranch("");
    setDeleteCandidate(null);
  }, [repository?.id]);

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
      if (event.key === "Escape") {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOnPointer);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnPointer);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [open]);

  const checkout = async (name: string) => {
    if (await state.checkoutBranch(name)) {
      setOpen(false);
      setQuery("");
    }
  };

  const create = async () => {
    if (newBranch.trim() !== "" && (await state.createBranch(newBranch))) {
      setCreating(false);
      setNewBranch("");
      setQuery("");
    }
  };

  const remove = async () => {
    if (
      deleteCandidate !== null &&
      (await state.deleteBranch(deleteCandidate))
    ) {
      setDeleteCandidate(null);
    }
  };

  return (
    <div className="branch-control" ref={root}>
      <button
        className="branch-indicator"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={`Current branch ${repository?.branch.displayName ?? "none"}`}
        disabled={repository === null}
        onClick={() => setOpen((current) => !current)}
      >
        <Icon name="branch" />
        <span className="switcher-copy">
          <span className="control-label">Current branch</span>
          <strong>{repository?.branch.displayName ?? "—"}</strong>
        </span>
        <Icon name="arrow" />
      </button>

      {open && repository !== null && (
        <div className="branch-popover" role="dialog" aria-label="Branches">
          <div className="branch-popover-header">
            <strong>Branches</strong>
            <button
              type="button"
              disabled={repository.branch.unborn || state.loading || state.busy}
              title={
                repository.branch.unborn
                  ? "Create the first commit before adding branches"
                  : "Create branch"
              }
              onClick={() => {
                setCreating(true);
                state.dismissError();
              }}
            >
              New branch
            </button>
          </div>

          {creating && (
            <form
              className="branch-create-form"
              onSubmit={(event) => {
                event.preventDefault();
                void create();
              }}
            >
              <input
                autoFocus
                aria-label="New branch name"
                value={newBranch}
                maxLength={255}
                placeholder="feature/name"
                disabled={state.busy}
                onChange={(event) => setNewBranch(event.target.value)}
              />
              <button type="submit" disabled={state.busy || newBranch.trim() === ""}>
                Create
              </button>
              <button
                type="button"
                disabled={state.busy}
                onClick={() => {
                  setCreating(false);
                  setNewBranch("");
                }}
              >
                Cancel
              </button>
            </form>
          )}

          <input
            className="branch-search"
            type="search"
            aria-label="Search branches"
            value={query}
            placeholder="Search branches"
            onChange={(event) => setQuery(event.target.value)}
          />

          {state.error !== null && (
            <div className="branch-error" role="alert">
              <strong>{state.error.summary}</strong>
              {state.error.detail !== null && <span>{state.error.detail}</span>}
              <button type="button" disabled={state.busy} onClick={state.reload}>
                Refresh
              </button>
            </div>
          )}

          {deleteCandidate !== null && (
            <div className="branch-delete-confirm" role="alertdialog">
              <strong>Delete {deleteCandidate}?</strong>
              <span>
                Branchcraft will proceed only if it is fully merged into HEAD.
              </span>
              <div>
                <button type="button" disabled={state.busy} onClick={() => void remove()}>
                  Delete safely
                </button>
                <button
                  type="button"
                  disabled={state.busy}
                  onClick={() => setDeleteCandidate(null)}
                >
                  Cancel
                </button>
              </div>
            </div>
          )}

          {state.loading ? (
            <div className="branch-loading" role="status">
              <span className="loading-spinner" /> Loading branches…
            </div>
          ) : (
            <div className="branch-lists">
              <section>
                <h3>Local</h3>
                <ul>
                  {localBranches.map((branch) => (
                    <BranchRow
                      key={`local:${branch.name}`}
                      branch={branch}
                      busy={state.busy}
                      onCheckout={(name) => void checkout(name)}
                      onDelete={setDeleteCandidate}
                    />
                  ))}
                </ul>
              </section>
              {remoteBranches.length > 0 && (
                <section>
                  <h3>Remote-tracking</h3>
                  <ul>
                    {remoteBranches.map((branch) => (
                      <BranchRow
                        key={`remote:${branch.name}`}
                        branch={branch}
                        busy={state.busy}
                        onCheckout={(name) => void checkout(name)}
                        onDelete={setDeleteCandidate}
                      />
                    ))}
                  </ul>
                </section>
              )}
              {visibleBranches.length === 0 && (
                <p className="branch-empty">No branches match this search.</p>
              )}
            </div>
          )}
        </div>
      )}
    </div>
  );
}
