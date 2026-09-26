import { useCallback, useEffect, useState } from "react";
import type { DiffService } from "../../services/diffService";
import type { FileDiff } from "../../types/diff";
import { isGitError, type GitError } from "../../types/repository";

interface FileDiffState {
  diff: FileDiff | null;
  error: GitError | null;
  loading: boolean;
  reload: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not load this diff.",
  detail: "The file may have changed while it was being inspected.",
};

export function useFileDiff(
  service: DiffService,
  repositoryPath: string | null,
  filePath: string | null,
  repositoryRevision = 0,
): FileDiffState {
  const [diff, setDiff] = useState<FileDiff | null>(null);
  const [error, setError] = useState<GitError | null>(null);
  const [loading, setLoading] = useState(false);
  const [revision, setRevision] = useState(0);

  useEffect(() => {
    let active = true;

    if (repositoryPath === null || filePath === null) {
      setDiff(null);
      setError(null);
      setLoading(false);
      return () => {
        active = false;
      };
    }

    setDiff(null);
    setError(null);
    setLoading(true);

    const load = async () => {
      try {
        const result = await service.getFileDiff(repositoryPath, filePath);
        if (active) {
          setDiff(result);
        }
      } catch (cause: unknown) {
        if (active) {
          setError(isGitError(cause) ? cause : unknownError);
        }
      } finally {
        if (active) {
          setLoading(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, [filePath, repositoryPath, repositoryRevision, revision, service]);

  const reload = useCallback(() => {
    setRevision((current) => current + 1);
  }, []);

  return { diff, error, loading, reload };
}
