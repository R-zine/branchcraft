import { useCallback, useEffect, useState } from "react";
import type { HistoryService } from "../../services/historyService";
import type { CommitHistory } from "../../types/history";
import { isGitError, type GitError } from "../../types/repository";

interface HistoryState {
  history: CommitHistory | null;
  loading: boolean;
  error: GitError | null;
  reload: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not load the commit history.",
  detail: "The repository may have changed while history was loading.",
};

export function useHistory(
  service: HistoryService,
  repositoryPath: string,
  repositoryRevision: number,
): HistoryState {
  const [history, setHistory] = useState<CommitHistory | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<GitError | null>(null);
  const [reloadToken, setReloadToken] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);

    const load = async () => {
      try {
        const result = await service.getHistory(repositoryPath);
        if (active) {
          setHistory(result);
        }
      } catch (cause: unknown) {
        if (active) {
          setHistory(null);
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
  }, [reloadToken, repositoryPath, repositoryRevision, service]);

  const reload = useCallback(() => {
    setReloadToken((current) => current + 1);
  }, []);

  return { history, loading, error, reload };
}
