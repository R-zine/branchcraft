export type GitHook =
  | "preCommit"
  | "prepareCommitMsg"
  | "commitMsg"
  | "postCommit";

export type HookEventKind =
  | "started"
  | "stdout"
  | "stderr"
  | "succeeded"
  | "failed";

export interface HookEvent {
  hook: GitHook;
  kind: HookEventKind;
  text: string | null;
  exitCode: number | null;
}
