export type DiffFileKind =
  | "added"
  | "modified"
  | "deleted"
  | "renamed"
  | "typeChanged"
  | "conflicted";

export type DiffPreviewStatus =
  | "ready"
  | "binary"
  | "tooLarge"
  | "unsupported";

export interface DiffFile {
  path: string;
  previousPath: string | null;
  kind: DiffFileKind;
  previewStatus: DiffPreviewStatus;
  oldContent: string | null;
  newContent: string | null;
  oldSize: number;
  newSize: number;
}

export type DiffLineKind = "context" | "added" | "deleted" | "noNewline";

export interface DiffLine {
  kind: DiffLineKind;
  oldLineNumber: number | null;
  newLineNumber: number | null;
  content: string;
}

export interface DiffHunk {
  header: string;
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
}

export interface FileDiff {
  file: DiffFile;
  hunks: DiffHunk[];
}
