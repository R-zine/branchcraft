import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { BranchService } from "../../services/branchService";
import type { BranchSummary } from "../../types/branches";
import type { RepositorySnapshot } from "../../types/repository";
import { BranchPicker } from "./BranchPicker";

const repository: RepositorySnapshot = {
  id: "/work/branchcraft",
  name: "branchcraft",
  path: "/work/branchcraft",
  branch: { displayName: "main", detached: false, unborn: false },
  files: [],
};

const branches: BranchSummary[] = [
  {
    name: "main",
    kind: "local",
    current: true,
    target: "1111111111111111",
    shortTarget: "11111111",
    upstream: "origin/main",
  },
  {
    name: "feature/work",
    kind: "local",
    current: false,
    target: "2222222222222222",
    shortTarget: "22222222",
    upstream: null,
  },
  {
    name: "origin/main",
    kind: "remote",
    current: false,
    target: "1111111111111111",
    shortTarget: "11111111",
    upstream: null,
  },
];

function service() {
  const listBranches = vi
    .fn<BranchService["listBranches"]>()
    .mockResolvedValue(branches);
  const createBranch = vi
    .fn<BranchService["createBranch"]>()
    .mockResolvedValue({ repository, branches });
  const checkoutBranch = vi
    .fn<BranchService["checkoutBranch"]>()
    .mockResolvedValue({
      repository: {
        ...repository,
        branch: { displayName: "feature/work", detached: false, unborn: false },
      },
      branches: branches.map((branch) => ({
        ...branch,
        current: branch.name === "feature/work",
      })),
    });
  const deleteBranch = vi
    .fn<BranchService["deleteBranch"]>()
    .mockResolvedValue({
      repository,
      branches: branches.filter((branch) => branch.name !== "feature/work"),
    });
  return {
    service: {
      listBranches,
      createBranch,
      checkoutBranch,
      deleteBranch,
    } satisfies BranchService,
    createBranch,
    checkoutBranch,
    deleteBranch,
  };
}

describe("BranchPicker", () => {
  it("searches and switches local branches while listing remotes", async () => {
    const branchService = service();
    const onUpdated = vi.fn();
    render(
      <BranchPicker
        repository={repository}
        service={branchService.service}
        onUpdated={onUpdated}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Current branch main" }));
    expect(
      await screen.findByRole("heading", { name: "Remote-tracking" }),
    ).toBeDefined();
    fireEvent.change(screen.getByRole("searchbox", { name: "Search branches" }), {
      target: { value: "feature" },
    });
    expect(screen.queryByText("origin/main")).toBeNull();
    fireEvent.click(
      screen.getByRole("button", { name: "Switch to branch feature/work" }),
    );

    await waitFor(() => {
      expect(branchService.checkoutBranch).toHaveBeenCalledWith(
        "/work/branchcraft",
        "feature/work",
      );
    });
    expect(onUpdated).toHaveBeenCalled();
  });

  it("creates branches and requires confirmation before safe deletion", async () => {
    const branchService = service();
    render(
      <BranchPicker
        repository={repository}
        service={branchService.service}
        onUpdated={vi.fn()}
      />,
    );

    fireEvent.click(screen.getByRole("button", { name: "Current branch main" }));
    await screen.findByText("feature/work");
    fireEvent.click(screen.getByRole("button", { name: "New branch" }));
    fireEvent.change(screen.getByRole("textbox", { name: "New branch name" }), {
      target: { value: "feature/new" },
    });
    fireEvent.click(screen.getByRole("button", { name: "Create" }));
    await waitFor(() => {
      expect(branchService.createBranch).toHaveBeenCalledWith(
        "/work/branchcraft",
        "feature/new",
      );
    });

    fireEvent.click(screen.getByRole("button", { name: "Delete feature/work" }));
    expect(screen.getByRole("alertdialog")).toBeDefined();
    expect(branchService.deleteBranch).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole("button", { name: "Delete safely" }));

    await waitFor(() => {
      expect(branchService.deleteBranch).toHaveBeenCalledWith(
        "/work/branchcraft",
        "feature/work",
      );
    });
  });
});
