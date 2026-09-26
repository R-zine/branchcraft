import { invoke } from "@tauri-apps/api/core";
import type { GitHubRepositoryList } from "../types/github";

export interface GitHubRepositoryService {
  listRepositories(): Promise<GitHubRepositoryList>;
}

export const tauriGitHubRepositoryService: GitHubRepositoryService = {
  listRepositories() {
    return invoke<GitHubRepositoryList>("list_github_repositories");
  },
};
