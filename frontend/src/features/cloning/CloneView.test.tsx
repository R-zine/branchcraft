import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { CloneService } from "../../services/cloneService";
import type { GitHubRepositoryService } from "../../services/githubRepositoryService";
import type { GitHubRepository } from "../../types/github";
import type { RepositorySnapshot } from "../../types/repository";
import { CloneView } from "./CloneView";

const repositorySnapshot: RepositorySnapshot = {
  id: "/work/branchcraft",
  name: "branchcraft",
  path: "/work/branchcraft",
  branch: { displayName: "main", detached: false, unborn: false },
  files: [],
};

const repositories: GitHubRepository[] = [
  {
    id: 1,
    name: "branchcraft",
    fullName: "octocat/branchcraft",
    ownerLogin: "octocat",
    description: "A focused Git client",
    private: false,
    fork: false,
    archived: false,
    cloneUrl: "https://github.com/octocat/branchcraft.git",
    defaultBranch: "main",
    language: "Rust",
    updatedAt: "2026-09-25T10:00:00Z",
  },
  {
    id: 2,
    name: "private-tool",
    fullName: "octocat/private-tool",
    ownerLogin: "octocat",
    description: "Internal tooling",
    private: true,
    fork: false,
    archived: false,
    cloneUrl: "https://github.com/octocat/private-tool.git",
    defaultBranch: "main",
    language: "TypeScript",
    updatedAt: "2026-09-24T10:00:00Z",
  },
];

function githubService(
  result: GitHubRepository[] = repositories,
): GitHubRepositoryService {
  return {
    listRepositories: vi.fn().mockResolvedValue({
      repositories: result,
      truncated: false,
    }),
  };
}

function cloneService() {
  const chooseDestination = vi
    .fn<CloneService["chooseDestination"]>()
    .mockResolvedValue("/work");
  const cloneRepository = vi
    .fn<CloneService["cloneRepository"]>()
    .mockResolvedValue(repositorySnapshot);
  return {
    service: { chooseDestination, cloneRepository } satisfies CloneService,
    chooseDestination,
    cloneRepository,
  };
}

describe("CloneView", () => {
  it("searches and filters accessible GitHub repositories", async () => {
    const cloning = cloneService();
    render(
      <CloneView
        githubService={githubService()}
        cloneService={cloning.service}
        onCloned={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    expect(await screen.findByText("octocat/branchcraft")).toBeDefined();
    expect(screen.getByText("octocat/private-tool")).toBeDefined();

    fireEvent.change(screen.getByRole("searchbox", { name: "Search GitHub repositories" }), {
      target: { value: "typescript" },
    });
    expect(screen.queryByText("octocat/branchcraft")).toBeNull();
    expect(screen.getByText("octocat/private-tool")).toBeDefined();

    fireEvent.change(screen.getByRole("searchbox", { name: "Search GitHub repositories" }), {
      target: { value: "" },
    });
    fireEvent.change(screen.getByRole("combobox", { name: "Filter GitHub repositories" }), {
      target: { value: "public" },
    });
    expect(screen.getByText("octocat/branchcraft")).toBeDefined();
    expect(screen.queryByText("octocat/private-tool")).toBeNull();
  });

  it("clones a selected GitHub repository with visible progress", async () => {
    const cloning = cloneService();
    let finishClone: (repository: RepositorySnapshot) => void = () => undefined;
    const pendingClone = new Promise<RepositorySnapshot>((resolve) => {
      finishClone = resolve;
    });
    cloning.cloneRepository.mockImplementation((_url, _destination, onProgress) => {
      onProgress({ phase: "receiving", current: 3, total: 8, bytes: 2048 });
      return pendingClone;
    });
    const onCloned = vi.fn();
    render(
      <CloneView
        githubService={githubService()}
        cloneService={cloning.service}
        onCloned={onCloned}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Clone octocat/branchcraft" }));

    expect(await screen.findByText("Receiving objects 3 of 8")).toBeDefined();
    expect(cloning.chooseDestination).toHaveBeenCalledOnce();
    expect(cloning.cloneRepository).toHaveBeenCalledWith(
      "https://github.com/octocat/branchcraft.git",
      "/work",
      expect.any(Function),
    );
    act(() => finishClone(repositorySnapshot));
    await waitFor(() => expect(onCloned).toHaveBeenCalledWith(repositorySnapshot));
  });

  it("clones a trimmed repository URL", async () => {
    const cloning = cloneService();
    const onCloned = vi.fn();
    render(
      <CloneView
        githubService={githubService([])}
        cloneService={cloning.service}
        onCloned={onCloned}
        onBack={vi.fn()}
      />,
    );

    fireEvent.change(screen.getByLabelText("Repository URL"), {
      target: { value: "  https://github.com/octocat/project.git  " },
    });
    fireEvent.click(screen.getByRole("button", { name: "Choose destination and clone" }));

    await waitFor(() => {
      expect(cloning.cloneRepository).toHaveBeenCalledWith(
        "https://github.com/octocat/project.git",
        "/work",
        expect.any(Function),
      );
    });
    expect(onCloned).toHaveBeenCalledWith(repositorySnapshot);
  });

  it("keeps clone by URL available when GitHub discovery needs sign-in", async () => {
    const github: GitHubRepositoryService = {
      listRepositories: vi.fn().mockRejectedValue({
        kind: "authentication",
        summary: "GitHub authentication is required.",
        detail: "Sign in to GitHub, then retry the operation.",
      }),
    };
    const cloning = cloneService();
    render(
      <CloneView
        githubService={github}
        cloneService={cloning.service}
        onCloned={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    expect(await screen.findByText("GitHub authentication is required.")).toBeDefined();
    expect(screen.getByLabelText("Repository URL")).toBeDefined();
  });

  it("recovers when the native destination picker fails", async () => {
    const cloning = cloneService();
    cloning.chooseDestination.mockRejectedValue(new Error("dialog unavailable"));
    render(
      <CloneView
        githubService={githubService()}
        cloneService={cloning.service}
        onCloned={vi.fn()}
        onBack={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Clone octocat/branchcraft" }));

    expect(
      await screen.findByText("Branchcraft could not clone the repository."),
    ).toBeDefined();
    expect(cloning.cloneRepository).not.toHaveBeenCalled();
    expect(
      screen.getByRole<HTMLButtonElement>("button", {
        name: "Clone octocat/branchcraft",
      }).disabled,
    ).toBe(false);
  });
});
