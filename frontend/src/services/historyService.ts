import { invoke } from "@tauri-apps/api/core";
import type { CommitHistory } from "../types/history";

export interface HistoryService {
  getHistory(repositoryPath: string): Promise<CommitHistory>;
}

export const tauriHistoryService: HistoryService = {
  getHistory(repositoryPath) {
    return invoke<CommitHistory>("get_commit_history", {
      request: { repositoryPath },
    });
  },
};
