import { useCallback, useState } from "react";
import type { RepositoryService } from "../../services/repositoryService";
import {
  isGitError,
  type GitError,
  type RepositorySnapshot,
} from "../../types/repository";

interface RepositoryState {
  repository: RepositorySnapshot | null;
  selectedPath: string | null;
  error: GitError | null;
  loading: boolean;
  revision: number;
  openRepository: () => Promise<void>;
  refreshRepository: () => Promise<void>;
  selectPath: (path: string) => void;
  updateRepository: (snapshot: RepositorySnapshot) => void;
  dismissError: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not open this repository.",
  detail: "An unexpected application error occurred.",
};

export function useRepository(service: RepositoryService): RepositoryState {
  const [repository, setRepository] = useState<RepositorySnapshot | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [error, setError] = useState<GitError | null>(null);
  const [loading, setLoading] = useState(false);
  const [revision, setRevision] = useState(0);

  const updateRepository = useCallback((snapshot: RepositorySnapshot) => {
    setRepository(snapshot);
    setSelectedPath((current) =>
      snapshot.files.some((file) => file.path === current)
        ? current
        : (snapshot.files[0]?.path ?? null),
    );
    setRevision((current) => current + 1);
  }, []);

  const run = useCallback(
    async (operation: () => Promise<RepositorySnapshot>) => {
      setLoading(true);
      setError(null);
      try {
        updateRepository(await operation());
      } catch (cause: unknown) {
        setError(isGitError(cause) ? cause : unknownError);
      } finally {
        setLoading(false);
      }
    },
    [updateRepository],
  );

  const openRepository = useCallback(async () => {
    try {
      const path = await service.chooseDirectory();
      if (path !== null) {
        await run(() => service.openRepository(path));
      }
    } catch (cause: unknown) {
      setError(isGitError(cause) ? cause : unknownError);
    }
  }, [run, service]);

  const refreshRepository = useCallback(async () => {
    if (repository !== null) {
      await run(() => service.refreshRepository(repository.path));
    }
  }, [repository, run, service]);

  return {
    repository,
    selectedPath,
    error,
    loading,
    revision,
    openRepository,
    refreshRepository,
    selectPath: setSelectedPath,
    updateRepository,
    dismissError: () => setError(null),
  };
}

