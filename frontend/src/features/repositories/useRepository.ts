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
  openRepository: () => Promise<void>;
  refreshRepository: () => Promise<void>;
  selectPath: (path: string) => void;
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

  const run = useCallback(async (operation: () => Promise<RepositorySnapshot>) => {
    setLoading(true);
    setError(null);
    try {
      const snapshot = await operation();
      setRepository(snapshot);
      setSelectedPath((current) =>
        snapshot.files.some((file) => file.path === current)
          ? current
          : (snapshot.files[0]?.path ?? null),
      );
    } catch (cause: unknown) {
      setError(isGitError(cause) ? cause : unknownError);
    } finally {
      setLoading(false);
    }
  }, []);

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
    openRepository,
    refreshRepository,
    selectPath: setSelectedPath,
    dismissError: () => setError(null),
  };
}

