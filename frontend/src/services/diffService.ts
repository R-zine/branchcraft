import { invoke } from "@tauri-apps/api/core";
import type { FileDiff } from "../types/diff";

export interface DiffService {
  getFileDiff(repositoryPath: string, filePath: string): Promise<FileDiff>;
}

export const tauriDiffService: DiffService = {
  getFileDiff(repositoryPath, filePath) {
    return invoke<FileDiff>("get_file_diff", {
      request: { repositoryPath, filePath },
    });
  },
};
