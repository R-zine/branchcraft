import { invoke } from "@tauri-apps/api/core";
import type { RepositorySnapshot } from "../types/repository";

export interface StagingService {
  stageFile(repositoryPath: string, filePath: string): Promise<RepositorySnapshot>;
  unstageFile(repositoryPath: string, filePath: string): Promise<RepositorySnapshot>;
  stageAll(repositoryPath: string): Promise<RepositorySnapshot>;
  unstageAll(repositoryPath: string): Promise<RepositorySnapshot>;
}

const fileRequest = (repositoryPath: string, filePath: string) => ({
  request: { repositoryPath, filePath },
});

const repositoryRequest = (repositoryPath: string) => ({
  request: { repositoryPath },
});

export const tauriStagingService: StagingService = {
  stageFile(repositoryPath, filePath) {
    return invoke<RepositorySnapshot>("stage_file", fileRequest(repositoryPath, filePath));
  },

  unstageFile(repositoryPath, filePath) {
    return invoke<RepositorySnapshot>("unstage_file", fileRequest(repositoryPath, filePath));
  },

  stageAll(repositoryPath) {
    return invoke<RepositorySnapshot>("stage_all", repositoryRequest(repositoryPath));
  },

  unstageAll(repositoryPath) {
    return invoke<RepositorySnapshot>("unstage_all", repositoryRequest(repositoryPath));
  },
};
