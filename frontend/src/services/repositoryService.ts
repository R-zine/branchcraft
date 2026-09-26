import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { RepositorySnapshot } from "../types/repository";

export interface RepositoryService {
  chooseDirectory(): Promise<string | null>;
  openRepository(path: string): Promise<RepositorySnapshot>;
  refreshRepository(path: string): Promise<RepositorySnapshot>;
}

const request = (path: string): { request: { path: string } } => ({
  request: { path },
});

export const tauriRepositoryService: RepositoryService = {
  async chooseDirectory() {
    const selection = await open({
      title: "Open a Git repository",
      directory: true,
      multiple: false,
    });
    return typeof selection === "string" ? selection : null;
  },

  openRepository(path) {
    return invoke<RepositorySnapshot>("open_repository", request(path));
  },

  refreshRepository(path) {
    return invoke<RepositorySnapshot>("refresh_repository", request(path));
  },
};

