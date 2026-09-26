import type { RepositorySnapshot } from "./repository";

export type BranchKind = "local" | "remote";

export interface BranchSummary {
  name: string;
  kind: BranchKind;
  current: boolean;
  target: string;
  shortTarget: string;
  upstream: string | null;
}

export interface BranchOperationResult {
  repository: RepositorySnapshot;
  branches: BranchSummary[];
}
