export type ChangeKind =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "typeChanged"
  | "untracked";

export interface BranchInfo {
  displayName: string;
  detached: boolean;
  unborn: boolean;
}

export interface FileStatus {
  path: string;
  previousPath: string | null;
  staged: ChangeKind | null;
  unstaged: ChangeKind | null;
  conflicted: boolean;
}

export interface RepositorySnapshot {
  id: string;
  name: string;
  path: string;
  branch: BranchInfo;
  files: FileStatus[];
}

export interface CommitInfo {
  id: string;
  shortId: string;
  summary: string;
}

export interface CommitResult {
  commit: CommitInfo;
  repository: RepositorySnapshot;
}

export type GitErrorKind =
  | "notRepository"
  | "permissionDenied"
  | "repositoryNotFound"
  | "invalidPath"
  | "invalidInput"
  | "conflict"
  | "dirtyWorkingTree"
  | "nothingStaged"
  | "missingIdentity"
  | "invalidReference"
  | "branchAlreadyExists"
  | "branchNotFound"
  | "unsafeOperation"
  | "unknown";

export interface GitError {
  kind: GitErrorKind;
  summary: string;
  detail: string | null;
}

export function isGitError(value: unknown): value is GitError {
  if (typeof value !== "object" || value === null) {
    return false;
  }

  return "kind" in value && "summary" in value;
}

