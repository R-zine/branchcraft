import { describe, expect, it } from "vitest";
import type { FileStatus } from "../../types/repository";
import { statusCode, statusLabel, statusTone } from "./statusPresentation";

const file = (override: Partial<FileStatus>): FileStatus => ({
  path: "README.md",
  previousPath: null,
  staged: null,
  unstaged: null,
  conflicted: false,
  ...override,
});

describe("status presentation", () => {
  it("describes a mixed staged and unstaged file", () => {
    const mixed = file({ staged: "modified", unstaged: "modified" });

    expect(statusCode(mixed)).toBe("M");
    expect(statusLabel(mixed)).toBe("Staged modified, Modified");
    expect(statusTone(mixed)).toBe("warning");
  });

  it("gives conflicts priority", () => {
    const conflict = file({ conflicted: true, unstaged: "modified" });

    expect(statusCode(conflict)).toBe("U");
    expect(statusLabel(conflict)).toBe("Conflicted");
    expect(statusTone(conflict)).toBe("danger");
  });
});

