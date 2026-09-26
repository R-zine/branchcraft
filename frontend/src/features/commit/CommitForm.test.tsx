import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CommitService } from "../../services/commitService";
import type { RepositorySnapshot } from "../../types/repository";
import { CommitForm } from "./CommitForm";

const stagedRepository: RepositorySnapshot = {
  id: "/work/branchcraft",
  name: "branchcraft",
  path: "/work/branchcraft",
  branch: { displayName: "main", detached: false, unborn: false },
  files: [
    {
      path: "README.md",
      previousPath: null,
      staged: "modified",
      unstaged: null,
      conflicted: false,
    },
  ],
};

const cleanRepository: RepositorySnapshot = {
  ...stagedRepository,
  files: [],
};

describe("CommitForm", () => {
  it("validates the summary and staged changes before calling the backend", async () => {
    const createCommit = vi.fn<CommitService["createCommit"]>();
    const { rerender } = render(
      <CommitForm
        repository={stagedRepository}
        service={{ createCommit }}
        disabled={false}
        onCommitted={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Commit to main" }));
    expect(await screen.findByText("Enter a commit summary.")).toBeDefined();
    expect(createCommit).not.toHaveBeenCalled();

    rerender(
      <CommitForm
        repository={cleanRepository}
        service={{ createCommit }}
        disabled={false}
        onCommitted={vi.fn()}
      />,
    );
    fireEvent.change(screen.getByRole("textbox", { name: "Summary" }), {
      target: { value: "A summary" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Commit to main" }));
    expect(
      await screen.findByText("There are no staged changes to commit."),
    ).toBeDefined();
    expect(createCommit).not.toHaveBeenCalled();
  });

  it("creates a commit and reports its short id", async () => {
    const createCommit = vi.fn<CommitService["createCommit"]>().mockResolvedValue({
      commit: {
        id: "1234567890abcdef",
        shortId: "12345678",
        summary: "Update README",
      },
      repository: cleanRepository,
    });
    const onCommitted = vi.fn();
    render(
      <CommitForm
        repository={stagedRepository}
        service={{ createCommit }}
        disabled={false}
        onCommitted={onCommitted}
      />,
    );

    fireEvent.change(screen.getByRole("textbox", { name: "Summary" }), {
      target: { value: "Update README" },
    });
    fireEvent.change(screen.getByRole("textbox", { name: "Description" }), {
      target: { value: "Document the change." },
    });
    fireEvent.click(screen.getByRole("button", { name: "Commit to main" }));

    await waitFor(() => {
      expect(createCommit).toHaveBeenCalledWith(
        "/work/branchcraft",
        "Update README",
        "Document the change.",
      );
    });
    expect(onCommitted).toHaveBeenCalledWith(cleanRepository);
    expect(screen.getByText("Committed 12345678")).toBeDefined();
  });

  it("shows a clear missing-identity error", async () => {
    const createCommit = vi
      .fn<CommitService["createCommit"]>()
      .mockRejectedValue({
        kind: "missingIdentity",
        summary: "Git needs your name and email before committing.",
        detail: "Configure user.name and user.email and try again.",
      });
    render(
      <CommitForm
        repository={stagedRepository}
        service={{ createCommit }}
        disabled={false}
        onCommitted={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByRole("textbox", { name: "Summary" }), {
      target: { value: "Update README" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Commit to main" }));

    expect(
      await screen.findByText("Git needs your name and email before committing."),
    ).toBeDefined();
  });
});
