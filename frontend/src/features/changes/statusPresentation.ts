import type { ChangeKind, FileStatus } from "../../types/repository";

const labels: Record<ChangeKind, string> = {
  added: "Added",
  modified: "Modified",
  deleted: "Deleted",
  renamed: "Renamed",
  typeChanged: "Type changed",
  untracked: "Untracked",
};

const codes: Record<ChangeKind, string> = {
  added: "A",
  modified: "M",
  deleted: "D",
  renamed: "R",
  typeChanged: "T",
  untracked: "?",
};

export function statusLabel(file: FileStatus): string {
  if (file.conflicted) {
    return "Conflicted";
  }
  const parts: string[] = [];
  if (file.staged !== null) {
    parts.push(`Staged ${labels[file.staged].toLowerCase()}`);
  }
  if (file.unstaged !== null) {
    parts.push(labels[file.unstaged]);
  }
  return parts.join(", ");
}

export function statusCode(file: FileStatus): string {
  if (file.conflicted) {
    return "U";
  }
  return file.unstaged !== null
    ? codes[file.unstaged]
    : file.staged !== null
      ? codes[file.staged]
      : "";
}

export function statusTone(file: FileStatus): string {
  if (file.conflicted || file.unstaged === "deleted" || file.staged === "deleted") {
    return "danger";
  }
  if (file.unstaged === "modified" || file.staged === "modified") {
    return "warning";
  }
  return "success";
}

