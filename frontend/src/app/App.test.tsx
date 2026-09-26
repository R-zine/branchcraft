import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { DiffService } from "../services/diffService";
import type { RepositoryService } from "../services/repositoryService";
import type { FileDiff } from "../types/diff";
import type { RepositorySnapshot } from "../types/repository";
import { App } from "./App";

vi.mock("../features/diff/MonacoDiffEditor", () => ({
  MonacoDiffEditor: () => <div>Rendered diff</div>,
}));

const repository: RepositorySnapshot = {
  id: "/work/branchcraft",
  name: "branchcraft",
  path: "/work/branchcraft",
  branch: { displayName: "main", detached: false, unborn: false },
  files: [
    {
      path: "src/main.rs",
      previousPath: null,
      staged: null,
      unstaged: "modified",
      conflicted: false,
    },
  ],
};

const fileDiff: FileDiff = {
  file: {
    path: "src/main.rs",
    previousPath: null,
    kind: "modified",
    previewStatus: "ready",
    oldContent: "fn main() {}\n",
    newContent: "fn main() { println!(\"Branchcraft\"); }\n",
    oldSize: 13,
    newSize: 42,
  },
  hunks: [],
};

function diffService(): DiffService {
  return {
    getFileDiff: vi.fn().mockResolvedValue(fileDiff),
  };
}

function service() {
  const chooseDirectory = vi
    .fn<RepositoryService["chooseDirectory"]>()
    .mockResolvedValue("/work/branchcraft");
  const openRepository = vi
    .fn<RepositoryService["openRepository"]>()
    .mockResolvedValue(repository);
  const refreshRepository = vi
    .fn<RepositoryService["refreshRepository"]>()
    .mockResolvedValue(repository);

  return {
    repositoryService: {
      chooseDirectory,
      openRepository,
      refreshRepository,
    } satisfies RepositoryService,
    chooseDirectory,
    openRepository,
  };
}

describe("Branchcraft app", () => {
  it("opens a selected repository and renders its status", async () => {
    const { repositoryService, openRepository } = service();
    render(
      <App
        repositoryService={repositoryService}
        diffService={diffService()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: /^open repository$/i }),
    );

    await waitFor(() => {
      expect(screen.getByText("src/main.rs")).toBeDefined();
    });
    expect(screen.getByText("main")).toBeDefined();
    expect(openRepository).toHaveBeenCalledWith("/work/branchcraft");
  });

  it("keeps the welcome view when the directory dialog is cancelled", async () => {
    const { repositoryService, chooseDirectory, openRepository } = service();
    chooseDirectory.mockResolvedValue(null);
    render(
      <App
        repositoryService={repositoryService}
        diffService={diffService()}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: /^open repository$/i }),
    );

    await waitFor(() => {
      expect(chooseDirectory).toHaveBeenCalledOnce();
    });
    expect(openRepository).not.toHaveBeenCalled();
    expect(screen.getByRole("heading", { name: "Open a local repository" })).toBeDefined();
  });
});

