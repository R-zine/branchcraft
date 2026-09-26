export type CloneProgressPhase =
  | "connecting"
  | "receiving"
  | "checkout"
  | "completed";

export interface CloneProgress {
  phase: CloneProgressPhase;
  current: number;
  total: number;
  bytes: number;
}
