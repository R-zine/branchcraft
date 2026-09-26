import { BrandMark } from "../../components/BrandMark";
import { Icon } from "../../components/Icon";
import type { RepositorySnapshot } from "../../types/repository";

interface RepositoryHeaderProps {
  repository: RepositorySnapshot | null;
  loading: boolean;
  onOpen: () => void;
  onRefresh: () => void;
}

export function RepositoryHeader({
  repository,
  loading,
  onOpen,
  onRefresh,
}: RepositoryHeaderProps) {
  return (
    <header className="app-header">
      <div className="wordmark" aria-label="Branchcraft">
        <BrandMark />
        <span>Branchcraft</span>
      </div>

      <button className="repository-switcher" type="button" onClick={onOpen}>
        <Icon name="folder" />
        <span className="switcher-copy">
          <span className="control-label">Repository</span>
          <strong>{repository?.name ?? "Choose a repository"}</strong>
        </span>
        <Icon name="arrow" />
      </button>

      <div className="header-spacer" />

      <div className="branch-indicator">
        <Icon name="branch" />
        <span className="switcher-copy">
          <span className="control-label">Current branch</span>
          <strong>{repository?.branch.displayName ?? "—"}</strong>
        </span>
      </div>

      <button
        aria-label="Refresh repository"
        className="refresh-button"
        type="button"
        disabled={repository === null || loading}
        onClick={onRefresh}
      >
        <Icon name="refresh" />
        <span>{loading ? "Refreshing…" : "Refresh"}</span>
      </button>
    </header>
  );
}

