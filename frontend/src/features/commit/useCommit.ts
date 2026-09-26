import { useEffect, useRef, useState } from "react";
import type { CommitService } from "../../services/commitService";
import type {
  CommitInfo,
  GitError,
  RepositorySnapshot,
} from "../../types/repository";
import { isGitError } from "../../types/repository";

interface CommitState {
  summary: string;
  description: string;
  committing: boolean;
  error: GitError | null;
  lastCommit: CommitInfo | null;
  setSummary: (value: string) => void;
  setDescription: (value: string) => void;
  submit: () => Promise<void>;
  dismissError: () => void;
}

const unknownError: GitError = {
  kind: "unknown",
  summary: "Branchcraft could not create the commit.",
  detail: "The repository may have changed during the operation.",
};

export function useCommit(
  service: CommitService,
  repositoryPath: string,
  stagedCount: number,
  onCommitted: (snapshot: RepositorySnapshot) => void,
): CommitState {
  const [summary, updateSummary] = useState("");
  const [description, updateDescription] = useState("");
  const [committing, setCommitting] = useState(false);
  const [error, setError] = useState<GitError | null>(null);
  const [lastCommit, setLastCommit] = useState<CommitInfo | null>(null);
  const active = useRef(true);
  const inFlight = useRef(false);

  useEffect(
    () => () => {
      active.current = false;
    },
    [],
  );

  const setSummary = (value: string) => {
    updateSummary(value);
    setError(null);
    setLastCommit(null);
  };
  const setDescription = (value: string) => {
    updateDescription(value);
    setError(null);
    setLastCommit(null);
  };

  const submit = async () => {
    if (inFlight.current) {
      return;
    }
    if (summary.trim() === "") {
      setError({
        kind: "invalidInput",
        summary: "Enter a commit summary.",
        detail: "The summary is required before a commit can be created.",
      });
      return;
    }
    if (stagedCount === 0) {
      setError({
        kind: "nothingStaged",
        summary: "There are no staged changes to commit.",
        detail: "Stage at least one changed file and try again.",
      });
      return;
    }

    inFlight.current = true;
    setCommitting(true);
    setError(null);
    setLastCommit(null);
    try {
      const result = await service.createCommit(repositoryPath, summary, description);
      if (active.current) {
        onCommitted(result.repository);
        setLastCommit(result.commit);
        updateSummary("");
        updateDescription("");
      }
    } catch (cause: unknown) {
      if (active.current) {
        setError(isGitError(cause) ? cause : unknownError);
      }
    } finally {
      inFlight.current = false;
      if (active.current) {
        setCommitting(false);
      }
    }
  };

  return {
    summary,
    description,
    committing,
    error,
    lastCommit,
    setSummary,
    setDescription,
    submit,
    dismissError: () => setError(null),
  };
}
