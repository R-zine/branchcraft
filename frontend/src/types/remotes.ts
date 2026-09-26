import type { RepositorySnapshot } from "./repository";

export type RemoteKind = "gitHubHttps" | "local" | "unsupported";
export type RemoteOperation = "fetch" | "pull" | "push";
export type RemoteProgressPhase =
  | "connecting"
  | "transferring"
  | "updating"
  | "merging"
  | "completed";
export type RemoteOutcome =
  | "fetched"
  | "upToDate"
  | "fastForwarded"
  | "merged"
  | "pushed";

export interface RemoteInfo {
  name: string;
  displayUrl: string;
  kind: RemoteKind;
  tracking: boolean;
  ahead: number;
  behind: number;
}

export interface RemoteProgress {
  operation: RemoteOperation;
  phase: RemoteProgressPhase;
  remoteName: string;
  current: number;
  total: number;
  bytes: number;
}

export interface RemoteOperationResult {
  repository: RepositorySnapshot;
  remote: RemoteInfo;
  outcome: RemoteOutcome;
}
