import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { FileStatus } from "../../types/repository";
import { ChangesPanel } from "./ChangesPanel";

const unstaged: FileStatus = {
  path: "src/main.rs",
  previousPath: null,
  staged: null,
  unstaged: "modified",
  conflicted: false,
};

const staged: FileStatus = {
  path: "README.md",
  previousPath: null,
  staged: "modified",
  unstaged: null,
  conflicted: false,
};

function handlers() {
  return {
    onSelect: vi.fn(),
    onStageFile: vi.fn(),
    onUnstageFile: vi.fn(),
    onStageAll: vi.fn(),
    onUnstageAll: vi.fn(),
    onDismissError: vi.fn(),
  };
}

describe("ChangesPanel staging controls", () => {
  it("stages and unstages individual files", () => {
    const callbacks = handlers();
    render(
      <ChangesPanel
        files={[unstaged, staged]}
        selectedPath={unstaged.path}
        busy={false}
        pendingPath={null}
        error={null}
        {...callbacks}
      />,
    );

    fireEvent.click(screen.getByRole("checkbox", { name: "Stage src/main.rs" }));
    fireEvent.click(screen.getByRole("checkbox", { name: "Unstage README.md" }));

    expect(callbacks.onStageFile).toHaveBeenCalledWith("src/main.rs");
    expect(callbacks.onUnstageFile).toHaveBeenCalledWith("README.md");
  });

  it("exposes independent stage-all and unstage-all actions", () => {
    const callbacks = handlers();
    render(
      <ChangesPanel
        files={[unstaged, staged]}
        selectedPath={null}
        busy={false}
        pendingPath={null}
        error={null}
        {...callbacks}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Stage all" }));
    fireEvent.click(screen.getByRole("button", { name: "Unstage all" }));

    expect(callbacks.onStageAll).toHaveBeenCalledOnce();
    expect(callbacks.onUnstageAll).toHaveBeenCalledOnce();
  });
});
