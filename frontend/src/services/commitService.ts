import { Channel, invoke } from "@tauri-apps/api/core";
import type { HookEvent } from "../types/hooks";
import type { CommitResult } from "../types/repository";

export interface CommitService {
  createCommit(
    repositoryPath: string,
    summary: string,
    description: string,
    onHookEvent: (event: HookEvent) => void,
  ): Promise<CommitResult>;
}

export const tauriCommitService: CommitService = {
  createCommit(repositoryPath, summary, description, onHookEvent) {
    const channel = new Channel<HookEvent>(onHookEvent);
    return invoke<CommitResult>("create_commit", {
      request: {
        repositoryPath,
        summary,
        description: description.trim() === "" ? null : description,
      },
      onHookEvent: channel,
    });
  },
};
