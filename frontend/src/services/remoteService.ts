import { Channel, invoke } from "@tauri-apps/api/core";
import type {
  RemoteInfo,
  RemoteOperationResult,
  RemoteProgress,
} from "../types/remotes";

export type ProgressHandler = (progress: RemoteProgress) => void;

export interface RemoteService {
  getPrimaryRemote(repositoryPath: string): Promise<RemoteInfo | null>;
  fetch(repositoryPath: string, onProgress: ProgressHandler): Promise<RemoteOperationResult>;
  pull(repositoryPath: string, onProgress: ProgressHandler): Promise<RemoteOperationResult>;
  push(repositoryPath: string, onProgress: ProgressHandler): Promise<RemoteOperationResult>;
}

const request = (repositoryPath: string) => ({ repositoryPath });

function run(
  command: "fetch_remote" | "pull_remote" | "push_remote",
  repositoryPath: string,
  onProgress: ProgressHandler,
) {
  const channel = new Channel<RemoteProgress>(onProgress);
  return invoke<RemoteOperationResult>(command, {
    request: request(repositoryPath),
    onProgress: channel,
  });
}

export const tauriRemoteService: RemoteService = {
  getPrimaryRemote(repositoryPath) {
    return invoke<RemoteInfo | null>("get_primary_remote", {
      request: request(repositoryPath),
    });
  },

  fetch(repositoryPath, onProgress) {
    return run("fetch_remote", repositoryPath, onProgress);
  },

  pull(repositoryPath, onProgress) {
    return run("pull_remote", repositoryPath, onProgress);
  },

  push(repositoryPath, onProgress) {
    return run("push_remote", repositoryPath, onProgress);
  },
};
