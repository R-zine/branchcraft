import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { DiffService } from "../../services/diffService";
import type { FileDiff } from "../../types/diff";
import type { FileStatus } from "../../types/repository";
import { DiffViewer } from "./DiffViewer";

vi.mock("./MonacoDiffEditor", () => ({
  MonacoDiffEditor: ({
    language,
    sideBySide,
  }: {
    language: string;
    sideBySide: boolean;
  }) => <div>{`Rendered ${sideBySide ? "split" : "inline"} ${language}`}</div>,
}));

const file: FileStatus = {
  path: "src/main.rs",
  previousPath: null,
  staged: null,
  unstaged: "modified",
  conflicted: false,
};

const readyDiff: FileDiff = {
  file: {
    path: "src/main.rs",
    previousPath: null,
    kind: "modified",
    previewStatus: "ready",
    oldContent: "fn main() {}\n",
    newContent: "fn main() { println!(\"hello\"); }\n",
    oldSize: 13,
    newSize: 35,
  },
  hunks: [],
};

function service(result: FileDiff): DiffService {
  return {
    getFileDiff: vi.fn().mockResolvedValue(result),
  };
}

describe("DiffViewer", () => {
  it("renders syntax-aware split and inline modes", async () => {
    render(
      <DiffViewer
        repositoryPath="/work/branchcraft"
        file={file}
        service={service(readyDiff)}
      />,
    );

    expect(await screen.findByText("Rendered split rust")).toBeDefined();
    fireEvent.click(screen.getByRole("button", { name: "Inline" }));
    expect(screen.getByText("Rendered inline rust")).toBeDefined();
  });

  it("shows a graceful binary-file fallback", async () => {
    const binaryDiff: FileDiff = {
      file: {
        ...readyDiff.file,
        path: "image.png",
        kind: "added",
        previewStatus: "binary",
        oldContent: null,
        newContent: null,
        oldSize: 0,
        newSize: 4096,
      },
      hunks: [],
    };
    render(
      <DiffViewer
        repositoryPath="/work/branchcraft"
        file={{ ...file, path: "image.png", unstaged: "untracked" }}
        service={service(binaryDiff)}
      />,
    );

    await waitFor(() => {
      expect(screen.getByRole("heading", { name: "Binary file" })).toBeDefined();
    });
    expect(screen.queryByText(/Rendered/)).toBeNull();
    expect(screen.getByText("0 B → 4.0 KiB")).toBeDefined();
  });
});
