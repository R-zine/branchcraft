import { useCallback, useEffect, useState } from "react";
import type { GitHubRepositoryService } from "../../services/githubRepositoryService";
import type { GitHubRepository } from "../../types/github";
import { isGitError, type GitError } from "../../types/repository";

interface GitHubRepositoriesState {
  repositories: GitHubRepository[];
  truncated: boolean;
  loading: boolean;
  error: GitError | null;
  reload: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "GitHub repositories could not be loaded.",
  detail: "Try again in a moment.",
};

export function useGitHubRepositories(
  service: GitHubRepositoryService,
): GitHubRepositoriesState {
  const [repositories, setRepositories] = useState<GitHubRepository[]>([]);
  const [truncated, setTruncated] = useState(false);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<GitError | null>(null);
  const [reloadToken, setReloadToken] = useState(0);

  useEffect(() => {
    let active = true;
    setLoading(true);
    setError(null);
    void service
      .listRepositories()
      .then((result) => {
        if (active) {
          setRepositories(result.repositories);
          setTruncated(result.truncated);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setRepositories([]);
          setTruncated(false);
          setError(isGitError(cause) ? cause : unknownError);
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [reloadToken, service]);

  const reload = useCallback(() => {
    setReloadToken((current) => current + 1);
  }, []);

  return { repositories, truncated, loading, error, reload };
}
