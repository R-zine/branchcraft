import { useMemo, useState } from "react";
import { Icon } from "../../components/Icon";
import type { CloneService } from "../../services/cloneService";
import type { GitHubRepositoryService } from "../../services/githubRepositoryService";
import type { CloneProgress } from "../../types/cloning";
import type { GitHubRepository } from "../../types/github";
import type { RepositorySnapshot } from "../../types/repository";
import { useClone } from "./useClone";
import { useGitHubRepositories } from "./useGitHubRepositories";

type RepositoryFilter = "all" | "public" | "private" | "archived";

interface CloneViewProps {
  githubService: GitHubRepositoryService;
  cloneService: CloneService;
  onCloned: (repository: RepositorySnapshot) => void;
  onBack: () => void;
}

function matchesFilter(repository: GitHubRepository, filter: RepositoryFilter): boolean {
  switch (filter) {
    case "public":
      return !repository.private;
    case "private":
      return repository.private;
    case "archived":
      return repository.archived;
    case "all":
      return true;
  }
}

function progressCopy(progress: CloneProgress): string {
  switch (progress.phase) {
    case "connecting":
      return "Connecting securely…";
    case "receiving":
      return progress.total > 0
        ? `Receiving objects ${String(progress.current)} of ${String(progress.total)}`
        : "Receiving objects…";
    case "checkout":
      return progress.total > 0
        ? `Checking out files ${String(progress.current)} of ${String(progress.total)}`
        : "Checking out files…";
    case "completed":
      return "Opening repository…";
  }
}

export function CloneView({
  githubService,
  cloneService,
  onCloned,
  onBack,
}: CloneViewProps) {
  const [query, setQuery] = useState("");
  const [filter, setFilter] = useState<RepositoryFilter>("all");
  const [cloneUrl, setCloneUrl] = useState("");
  const [activeName, setActiveName] = useState<string | null>(null);
  const github = useGitHubRepositories(githubService);
  const cloning = useClone(cloneService, onCloned);
  const normalizedQuery = query.trim().toLowerCase();
  const visibleRepositories = useMemo(
    () =>
      github.repositories.filter((repository) => {
        const searchable = [
          repository.fullName,
          repository.description ?? "",
          repository.language ?? "",
        ]
          .join(" ")
          .toLowerCase();
        return (
          matchesFilter(repository, filter) &&
          (normalizedQuery === "" || searchable.includes(normalizedQuery))
        );
      }),
    [filter, github.repositories, normalizedQuery],
  );

  const startClone = (sourceUrl: string, name: string) => {
    setActiveName(name);
    void cloning.clone(sourceUrl).finally(() => setActiveName(null));
  };

  return (
    <main className="clone-view">
      <div className="clone-view-header">
        <div>
          <span className="clone-eyebrow">Repository discovery</span>
          <h1>Clone a repository</h1>
          <p>Select an accessible GitHub repository or enter a credential-free HTTPS URL.</p>
        </div>
        <button type="button" disabled={cloning.busy} onClick={onBack}>
          Back
        </button>
      </div>

      <div className="clone-layout">
        <section className="github-browser" aria-labelledby="github-repositories-heading">
          <div className="clone-section-heading">
            <div>
              <h2 id="github-repositories-heading">Your GitHub repositories</h2>
              <span>{github.repositories.length} available</span>
            </div>
            <button type="button" disabled={github.loading || cloning.busy} onClick={github.reload}>
              <Icon name="refresh" /> Reload
            </button>
          </div>

          <div className="repository-filters">
            <input
              type="search"
              aria-label="Search GitHub repositories"
              placeholder="Search name, owner, language…"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
            />
            <select
              aria-label="Filter GitHub repositories"
              value={filter}
              onChange={(event) => setFilter(event.target.value as RepositoryFilter)}
            >
              <option value="all">All repositories</option>
              <option value="public">Public</option>
              <option value="private">Private</option>
              <option value="archived">Archived</option>
            </select>
          </div>

          {github.error !== null && (
            <div className="clone-error" role="alert">
              <strong>{github.error.summary}</strong>
              {github.error.detail !== null && <span>{github.error.detail}</span>}
              <button type="button" disabled={github.loading} onClick={github.reload}>Retry</button>
            </div>
          )}

          {github.loading ? (
            <div className="repository-browser-state" role="status">
              <span className="loading-spinner" /> Loading GitHub repositories…
            </div>
          ) : visibleRepositories.length === 0 && github.error === null ? (
            <div className="repository-browser-state">
              <strong>No repositories match</strong>
              <span>Change the search or visibility filter.</span>
            </div>
          ) : (
            <ul className="github-repository-list">
              {visibleRepositories.map((repository) => (
                <li key={repository.id}>
                  <div className="github-repository-copy">
                    <div>
                      <strong>{repository.fullName}</strong>
                      <span className={repository.private ? "private" : "public"}>
                        {repository.private ? "Private" : "Public"}
                      </span>
                      {repository.archived && <span>Archived</span>}
                      {repository.fork && <span>Fork</span>}
                    </div>
                    <p>{repository.description ?? "No description provided."}</p>
                    <small>
                      {repository.language ?? "Unknown language"} · {repository.defaultBranch}
                    </small>
                  </div>
                  <button
                    type="button"
                    aria-label={`Clone ${repository.fullName}`}
                    disabled={cloning.busy}
                    onClick={() => startClone(repository.cloneUrl, repository.fullName)}
                  >
                    {cloning.busy && activeName === repository.fullName ? "Cloning…" : "Clone"}
                  </button>
                </li>
              ))}
            </ul>
          )}

          {github.truncated && (
            <p className="repository-limit">Showing the 1,000 most recently active repositories.</p>
          )}
        </section>

        <aside className="clone-url-panel">
          <div className="clone-section-heading">
            <div>
              <h2>Clone by URL</h2>
              <span>GitHub or another public HTTPS repository</span>
            </div>
          </div>
          <form
            onSubmit={(event) => {
              event.preventDefault();
              if (cloneUrl.trim() !== "") {
                startClone(cloneUrl, cloneUrl.trim());
              }
            }}
          >
            <label htmlFor="clone-url">Repository URL</label>
            <input
              id="clone-url"
              type="url"
              spellCheck={false}
              value={cloneUrl}
              placeholder="https://github.com/owner/repository.git"
              disabled={cloning.busy}
              onChange={(event) => setCloneUrl(event.target.value)}
            />
            <button type="submit" disabled={cloning.busy || cloneUrl.trim() === ""}>
              Choose destination and clone
            </button>
          </form>
          <p className="clone-security-note">
            Branchcraft creates a repository-named folder inside the destination you choose. Credentials are never added to the URL.
          </p>

          {cloning.progress !== null && (
            <div className="clone-progress" role="status" aria-live="polite">
              <span className="loading-spinner" />
              <div>
                <strong>Cloning {activeName ?? "repository"}</strong>
                <span>{progressCopy(cloning.progress)}</span>
                <progress
                  max={Math.max(1, cloning.progress.total)}
                  value={cloning.progress.total > 0 ? cloning.progress.current : undefined}
                />
              </div>
            </div>
          )}

          {cloning.error !== null && (
            <div className="clone-error" role="alert">
              <strong>{cloning.error.summary}</strong>
              {cloning.error.detail !== null && <span>{cloning.error.detail}</span>}
              <button type="button" onClick={cloning.dismissError}>Dismiss</button>
            </div>
          )}
        </aside>
      </div>
    </main>
  );
}
