import { BrandMark } from "../../components/BrandMark";
import { Icon } from "../../components/Icon";
import type { RepositorySnapshot } from "../../types/repository";
import type { ReactNode } from "react";

interface RepositoryHeaderProps {
  repository: RepositorySnapshot | null;
  loading: boolean;
  branchControl: ReactNode;
  remoteControl: ReactNode;
  accountControl: ReactNode;
  onSettings: () => void;
  onOpen: () => void;
  onRefresh: () => void;
}

export function RepositoryHeader({
  repository,
  loading,
  branchControl,
  remoteControl,
  accountControl,
  onSettings,
  onOpen,
  onRefresh,
}: RepositoryHeaderProps) {
  return (
    <header className="app-header">
      <div className="wordmark" aria-label="Branchcraft">
        <BrandMark />
        <span>Branchcraft</span>
      </div>

      <button
        className="repository-switcher"
        type="button"
        title="Open repository (Ctrl+O)"
        onClick={onOpen}
      >
        <Icon name="folder" />
        <span className="switcher-copy">
          <span className="control-label">Repository</span>
          <strong>{repository?.name ?? "Choose a repository"}</strong>
        </span>
        <Icon name="arrow" />
      </button>

      <div className="header-spacer" />

      {branchControl}

      {remoteControl}

      <button
        aria-label="Refresh repository"
        className="refresh-button"
        type="button"
        disabled={repository === null || loading}
        title="Refresh repository (Ctrl+R)"
        onClick={onRefresh}
      >
        <Icon name="refresh" />
        <span>{loading ? "Refreshing…" : "Refresh"}</span>
      </button>

      <button
        aria-label="Open settings"
        className="refresh-button settings-button"
        type="button"
        title="Settings (Ctrl+,)"
        onClick={onSettings}
      >
        <Icon name="settings" />
      </button>

      {accountControl}
    </header>
  );
}

