export interface HistoryCommit {
  id: string;
  shortId: string;
  summary: string;
  description: string | null;
  authorName: string;
  authorEmail: string;
  authoredAt: number;
  authorOffsetMinutes: number;
  parentIds: string[];
}

export interface CommitHistory {
  commits: HistoryCommit[];
  truncated: boolean;
}
