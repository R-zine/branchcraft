import { invoke } from "@tauri-apps/api/core";
import type { CommitResult } from "../types/repository";

export interface CommitService {
  createCommit(
    repositoryPath: string,
    summary: string,
    description: string,
  ): Promise<CommitResult>;
}

export const tauriCommitService: CommitService = {
  createCommit(repositoryPath, summary, description) {
    return invoke<CommitResult>("create_commit", {
      request: {
        repositoryPath,
        summary,
        description: description.trim() === "" ? null : description,
      },
    });
  },
};
