import { useCallback, useEffect, useRef, useState } from "react";
import type { BranchService } from "../../services/branchService";
import type { BranchOperationResult, BranchSummary } from "../../types/branches";
import {
  isGitError,
  type GitError,
  type RepositorySnapshot,
} from "../../types/repository";

interface BranchesState {
  branches: BranchSummary[];
  loading: boolean;
  busy: boolean;
  error: GitError | null;
  reload: () => void;
  createBranch: (name: string) => Promise<boolean>;
  checkoutBranch: (name: string) => Promise<boolean>;
  deleteBranch: (name: string) => Promise<boolean>;
  dismissError: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not complete the branch operation.",
  detail: "The repository may have changed. Refresh the branch list and try again.",
};

export function useBranches(
  service: BranchService,
  repositoryPath: string | null,
  enabled: boolean,
  onUpdated: (snapshot: RepositorySnapshot) => void,
): BranchesState {
  const [branches, setBranches] = useState<BranchSummary[]>([]);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<GitError | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const activeRepository = useRef(repositoryPath);
  const operationId = useRef(0);
  const mutationId = useRef(0);
  const operationBusy = useRef(false);
  activeRepository.current = repositoryPath;

  useEffect(() => {
    mutationId.current += 1;
    operationBusy.current = false;
    setBusy(false);
  }, [repositoryPath]);

  useEffect(() => {
    if (!enabled || repositoryPath === null) {
      return undefined;
    }
    const currentOperation = operationId.current + 1;
    operationId.current = currentOperation;
    let active = true;
    setLoading(true);
    setError(null);

    const load = async () => {
      try {
        const result = await service.listBranches(repositoryPath);
        if (active && operationId.current === currentOperation) {
          setBranches(result);
        }
      } catch (cause: unknown) {
        if (active && operationId.current === currentOperation) {
          setError(isGitError(cause) ? cause : unknownError);
        }
      } finally {
        if (active && operationId.current === currentOperation) {
          setLoading(false);
        }
      }
    };

    void load();
    return () => {
      active = false;
    };
  }, [enabled, reloadToken, repositoryPath, service]);

  const run = useCallback(
    async (
      operation: (path: string) => Promise<BranchOperationResult>,
    ): Promise<boolean> => {
      if (repositoryPath === null || operationBusy.current) {
        return false;
      }
      const expectedRepository = repositoryPath;
      operationId.current += 1;
      const currentMutation = mutationId.current + 1;
      mutationId.current = currentMutation;
      operationBusy.current = true;
      setLoading(false);
      setBusy(true);
      setError(null);
      try {
        const result = await operation(repositoryPath);
        if (
          mutationId.current === currentMutation &&
          activeRepository.current === expectedRepository
        ) {
          setBranches(result.branches);
          onUpdated(result.repository);
          return true;
        }
      } catch (cause: unknown) {
        if (
          mutationId.current === currentMutation &&
          activeRepository.current === expectedRepository
        ) {
          setError(isGitError(cause) ? cause : unknownError);
        }
      } finally {
        if (mutationId.current === currentMutation) {
          operationBusy.current = false;
          setBusy(false);
        }
      }
      return false;
    },
    [onUpdated, repositoryPath],
  );

  return {
    branches,
    loading,
    busy,
    error,
    reload: () => setReloadToken((current) => current + 1),
    createBranch: (name) =>
      run((path) => service.createBranch(path, name.trim())),
    checkoutBranch: (name) =>
      run((path) => service.checkoutBranch(path, name)),
    deleteBranch: (name) => run((path) => service.deleteBranch(path, name)),
    dismissError: () => setError(null),
  };
}
