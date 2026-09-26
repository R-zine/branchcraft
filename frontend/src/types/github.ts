export interface GitHubRepository {
  id: number;
  name: string;
  fullName: string;
  ownerLogin: string;
  description: string | null;
  private: boolean;
  fork: boolean;
  archived: boolean;
  cloneUrl: string;
  defaultBranch: string;
  language: string | null;
  updatedAt: string;
}

export interface GitHubRepositoryList {
  repositories: GitHubRepository[];
  truncated: boolean;
}
