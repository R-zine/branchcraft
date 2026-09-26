import { useCallback, useEffect, useRef, useState } from "react";
import type { RemoteService } from "../../services/remoteService";
import {
  isGitError,
  type GitError,
  type RepositorySnapshot,
} from "../../types/repository";
import type {
  RemoteInfo,
  RemoteOperation,
  RemoteOperationResult,
  RemoteProgress,
} from "../../types/remotes";

interface RemotesState {
  remote: RemoteInfo | null;
  loading: boolean;
  busy: RemoteOperation | null;
  progress: RemoteProgress | null;
  error: GitError | null;
  notice: string | null;
  reload: () => void;
  fetch: () => Promise<boolean>;
  pull: () => Promise<boolean>;
  push: () => Promise<boolean>;
  dismissError: () => void;
  dismissNotice: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not finish the remote operation.",
  detail: "Refresh the repository and try again.",
};

function outcomeNotice(result: RemoteOperationResult): string {
  switch (result.outcome) {
    case "upToDate":
      return `${result.remote.name} is up to date.`;
    case "fastForwarded":
      return `Pulled the latest changes from ${result.remote.name}.`;
    case "merged":
      return `Merged changes from ${result.remote.name}.`;
    case "pushed":
      return `Pushed to ${result.remote.name}.`;
    case "fetched":
      return `Fetched updates from ${result.remote.name}.`;
  }
}

export function useRemotes(
  service: RemoteService,
  repositoryPath: string | null,
  repositoryRevision: number,
  onUpdated: (snapshot: RepositorySnapshot) => void,
): RemotesState {
  const [remote, setRemote] = useState<RemoteInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState<RemoteOperation | null>(null);
  const [progress, setProgress] = useState<RemoteProgress | null>(null);
  const [error, setError] = useState<GitError | null>(null);
  const [notice, setNotice] = useState<string | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const activeRepository = useRef(repositoryPath);
  const operationId = useRef(0);
  const operationBusy = useRef(false);
  activeRepository.current = repositoryPath;

  useEffect(() => {
    operationId.current += 1;
    operationBusy.current = false;
    setRemote(null);
    setBusy(null);
    setProgress(null);
    setError(null);
    setNotice(null);
  }, [repositoryPath]);

  useEffect(() => {
    if (repositoryPath === null || operationBusy.current) {
      return undefined;
    }
    const currentOperation = operationId.current + 1;
    operationId.current = currentOperation;
    let active = true;
    setLoading(true);

    void service
      .getPrimaryRemote(repositoryPath)
      .then((result) => {
        if (active && operationId.current === currentOperation) {
          setRemote(result);
        }
      })
      .catch((cause: unknown) => {
        if (active && operationId.current === currentOperation) {
          setError(isGitError(cause) ? cause : unknownError);
        }
      })
      .finally(() => {
        if (active && operationId.current === currentOperation) {
          setLoading(false);
        }
      });

    return () => {
      active = false;
    };
  }, [reloadToken, repositoryPath, repositoryRevision, service]);

  const run = useCallback(
    async (
      operation: RemoteOperation,
      execute: RemoteService[RemoteOperation],
    ): Promise<boolean> => {
      if (repositoryPath === null || operationBusy.current) {
        return false;
      }
      const expectedRepository = repositoryPath;
      const currentOperation = operationId.current + 1;
      operationId.current = currentOperation;
      operationBusy.current = true;
      setLoading(false);
      setBusy(operation);
      setProgress({
        operation,
        phase: "connecting",
        remoteName: remote?.name ?? "remote",
        current: 0,
        total: 0,
        bytes: 0,
      });
      setError(null);
      setNotice(null);
      try {
        const result = await execute(repositoryPath, (update) => {
          if (
            operationId.current === currentOperation &&
            activeRepository.current === expectedRepository
          ) {
            setProgress(update);
          }
        });
        if (
          operationId.current === currentOperation &&
          activeRepository.current === expectedRepository
        ) {
          setRemote(result.remote);
          setNotice(outcomeNotice(result));
          onUpdated(result.repository);
          return true;
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
          operationBusy.current = false;
          setBusy(null);
          setProgress(null);
        }
      }
      return false;
    },
    [onUpdated, remote?.name, repositoryPath],
  );

  return {
    remote,
    loading,
    busy,
    progress,
    error,
    notice,
    reload: () => setReloadToken((current) => current + 1),
    fetch: () => run("fetch", service.fetch.bind(service)),
    pull: () => run("pull", service.pull.bind(service)),
    push: () => run("push", service.push.bind(service)),
    dismissError: () => setError(null),
    dismissNotice: () => setNotice(null),
  };
}
