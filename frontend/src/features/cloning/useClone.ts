import { useCallback, useRef, useState } from "react";
import type { CloneService } from "../../services/cloneService";
import type { CloneProgress } from "../../types/cloning";
import {
  isGitError,
  type GitError,
  type RepositorySnapshot,
} from "../../types/repository";

interface CloneState {
  busy: boolean;
  progress: CloneProgress | null;
  error: GitError | null;
  clone: (sourceUrl: string) => Promise<boolean>;
  dismissError: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not clone the repository.",
  detail: "Check the repository URL and try again.",
};

export function useClone(
  service: CloneService,
  onCloned: (repository: RepositorySnapshot) => void,
): CloneState {
  const [busy, setBusy] = useState(false);
  const [progress, setProgress] = useState<CloneProgress | null>(null);
  const [error, setError] = useState<GitError | null>(null);
  const operationBusy = useRef(false);

  const clone = useCallback(
    async (sourceUrl: string): Promise<boolean> => {
      if (operationBusy.current || sourceUrl.trim() === "") {
        return false;
      }
      operationBusy.current = true;
      setBusy(true);
      setError(null);
      try {
        const destination = await service.chooseDestination();
        if (destination === null) {
          return false;
        }
        setProgress({
          phase: "connecting",
          current: 0,
          total: 0,
          bytes: 0,
        });
        const repository = await service.cloneRepository(
          sourceUrl.trim(),
          destination,
          setProgress,
        );
        onCloned(repository);
        return true;
      } catch (cause: unknown) {
        setError(isGitError(cause) ? cause : unknownError);
      } finally {
        operationBusy.current = false;
        setBusy(false);
        setProgress(null);
      }
      return false;
    },
    [onCloned, service],
  );

  return {
    busy,
    progress,
    error,
    clone,
    dismissError: () => setError(null),
  };
}
