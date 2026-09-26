import { useCallback, useEffect, useRef, useState } from "react";
import type { StagingService } from "../../services/stagingService";
import {
  isGitError,
  type GitError,
  type RepositorySnapshot,
} from "../../types/repository";

interface StagingState {
  busy: boolean;
  pendingPath: string | null;
  error: GitError | null;
  stageFile: (path: string) => Promise<void>;
  unstageFile: (path: string) => Promise<void>;
  stageAll: () => Promise<void>;
  unstageAll: () => Promise<void>;
  dismissError: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not update the staged changes.",
  detail: "The repository may have changed during the operation.",
};

export function useStaging(
  service: StagingService,
  repositoryPath: string | null,
  onUpdated: (snapshot: RepositorySnapshot) => void,
): StagingState {
  const [pendingPath, setPendingPath] = useState<string | null>(null);
  const [bulkPending, setBulkPending] = useState(false);
  const [error, setError] = useState<GitError | null>(null);
  const activeRepository = useRef(repositoryPath);
  const operationId = useRef(0);
  activeRepository.current = repositoryPath;

  useEffect(() => {
    operationId.current += 1;
    setPendingPath(null);
    setBulkPending(false);
    setError(null);
  }, [repositoryPath]);

  const run = useCallback(
    async (
      expectedRepository: string,
      path: string | null,
      operation: () => Promise<RepositorySnapshot>,
    ) => {
      const currentOperation = operationId.current + 1;
      operationId.current = currentOperation;
      setPendingPath(path);
      setBulkPending(path === null);
      setError(null);
      try {
        const snapshot = await operation();
        if (
          operationId.current === currentOperation &&
          activeRepository.current === expectedRepository
        ) {
          onUpdated(snapshot);
        }
      } catch (cause: unknown) {
        if (
          operationId.current === currentOperation &&
          activeRepository.current === expectedRepository
        ) {
          setError(isGitError(cause) ? cause : unknownError);
        }
      } finally {
        if (operationId.current === currentOperation) {
          setPendingPath(null);
          setBulkPending(false);
        }
      }
    },
    [onUpdated],
  );

  const withRepository = useCallback(
    async (
      operation: (path: string) => Promise<RepositorySnapshot>,
      path: string | null,
    ) => {
      if (repositoryPath !== null) {
        await run(repositoryPath, path, () => operation(repositoryPath));
      }
    },
    [repositoryPath, run],
  );

  return {
    busy: pendingPath !== null || bulkPending,
    pendingPath,
    error,
    stageFile: (path) =>
      withRepository(
        (repository) => service.stageFile(repository, path),
        path,
      ),
    unstageFile: (path) =>
      withRepository((repository) => service.unstageFile(repository, path), path),
    stageAll: () =>
      withRepository((repository) => service.stageAll(repository), null),
    unstageAll: () =>
      withRepository((repository) => service.unstageAll(repository), null),
    dismissError: () => setError(null),
  };
}
