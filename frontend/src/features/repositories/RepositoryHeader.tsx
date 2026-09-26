import { BrandMark } from "../../components/BrandMark";
import { Icon } from "../../components/Icon";
import type { RepositorySnapshot } from "../../types/repository";
import type { ReactNode } from "react";

interface RepositoryHeaderProps {
  repository: RepositorySnapshot | null;
  loading: boolean;
  branchControl: ReactNode;
  accountControl: ReactNode;
  onOpen: () => void;
  onRefresh: () => void;
}

export function RepositoryHeader({
  repository,
  loading,
  branchControl,
  accountControl,
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

      {branchControl}

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

      {accountControl}
    </header>
  );
}

