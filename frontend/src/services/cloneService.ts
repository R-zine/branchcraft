import { Channel, invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type { CloneProgress } from "../types/cloning";
import type { RepositorySnapshot } from "../types/repository";

export type CloneProgressHandler = (progress: CloneProgress) => void;

export interface CloneService {
  chooseDestination(): Promise<string | null>;
  cloneRepository(
    sourceUrl: string,
    destinationParent: string,
    onProgress: CloneProgressHandler,
  ): Promise<RepositorySnapshot>;
}

export const tauriCloneService: CloneService = {
  async chooseDestination() {
    const selection = await open({
      title: "Choose a parent folder for the cloned repository",
      directory: true,
      multiple: false,
    });
    return typeof selection === "string" ? selection : null;
  },

  cloneRepository(sourceUrl, destinationParent, onProgress) {
    const channel = new Channel<CloneProgress>(onProgress);
    return invoke<RepositorySnapshot>("clone_repository", {
      request: { sourceUrl, destinationParent },
      onProgress: channel,
    });
  },
};
