import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { RemoteService } from "../../services/remoteService";
import type { RepositorySnapshot } from "../../types/repository";
import type { RemoteInfo, RemoteOperationResult } from "../../types/remotes";
import { RemoteControls } from "./RemoteControls";

const repository: RepositorySnapshot = {
  id: "/work/branchcraft",
  name: "branchcraft",
  path: "/work/branchcraft",
  branch: { displayName: "main", detached: false, unborn: false },
  files: [],
};

const remote: RemoteInfo = {
  name: "origin",
  displayUrl: "https://github.com/example/branchcraft.git",
  kind: "gitHubHttps",
  tracking: true,
  ahead: 3,
  behind: 0,
};

function result(outcome: RemoteOperationResult["outcome"]): RemoteOperationResult {
  return { repository, remote: { ...remote, ahead: 0 }, outcome };
}

function service(primary: RemoteInfo | null = remote) {
  const fetch = vi.fn<RemoteService["fetch"]>().mockImplementation((_path, onProgress) => {
      onProgress({
        operation: "fetch",
        phase: "transferring",
        remoteName: "origin",
        current: 2,
        total: 5,
        bytes: 1024,
      });
      return Promise.resolve(result("upToDate"));
    });
  const pull = vi.fn<RemoteService["pull"]>().mockResolvedValue(result("fastForwarded"));
  const push = vi.fn<RemoteService["push"]>().mockResolvedValue(result("pushed"));
  return {
    remoteService: {
      getPrimaryRemote: vi.fn<RemoteService["getPrimaryRemote"]>().mockResolvedValue(primary),
      fetch,
      pull,
      push,
    } satisfies RemoteService,
    fetch,
    pull,
    push,
  };
}

describe("RemoteControls", () => {
  it("explains repositories without a remote", async () => {
    render(
      <RemoteControls
        repository={repository}
        repositoryRevision={1}
        service={service(null).remoteService}
        onUpdated={vi.fn()}
      />,
    );

    await waitFor(() => {
      expect(
        screen.getByRole("button", { name: "Fetch remote" }).hasAttribute("disabled"),
      ).toBe(true);
    });
    fireEvent.click(screen.getByRole("button", { name: "Remote actions" }));

    expect(screen.getByText("This repository has no configured remote.")).toBeDefined();
  });

  it("streams fetch progress and reports an up-to-date result", async () => {
    const { remoteService, fetch } = service();
    const onUpdated = vi.fn();
    let resolveFetch: (value: RemoteOperationResult) => void = () => undefined;
    const pendingFetch = new Promise<RemoteOperationResult>((resolve) => {
      resolveFetch = resolve;
    });
    fetch.mockImplementation((_path, onProgress) => {
      onProgress({
        operation: "fetch",
        phase: "transferring",
        remoteName: "origin",
        current: 2,
        total: 5,
        bytes: 1024,
      });
      return pendingFetch;
    });
    render(
      <RemoteControls
        repository={repository}
        repositoryRevision={1}
        service={remoteService}
        onUpdated={onUpdated}
      />,
    );

    const fetchButton = await screen.findByRole("button", { name: "Fetch origin" });
    fireEvent.click(fetchButton);

    expect(await screen.findByText("Transferring objects 2 of 5")).toBeDefined();
    act(() => resolveFetch(result("upToDate")));
    expect(await screen.findByText("origin is up to date.")).toBeDefined();
    expect(fetch).toHaveBeenCalledWith(
      repository.path,
      expect.any(Function),
    );
    expect(onUpdated).toHaveBeenCalledWith(repository);
  });

  it("shows actionable authentication failures", async () => {
    const { remoteService, fetch } = service();
    fetch.mockRejectedValue({
      kind: "authentication",
      summary: "Your GitHub sign-in has expired.",
      detail: "Sign in to GitHub again, then retry the operation.",
    });
    render(
      <RemoteControls
        repository={repository}
        repositoryRevision={1}
        service={remoteService}
        onUpdated={vi.fn()}
      />,
    );

    fireEvent.click(await screen.findByRole("button", { name: "Fetch origin" }));

    expect((await screen.findByRole("alert")).textContent).toContain("sign-in has expired");
    expect(screen.getByText("Sign in to GitHub again, then retry the operation.")).toBeDefined();
  });

  it("publishes the ahead count through push", async () => {
    const { remoteService, push } = service();
    render(
      <RemoteControls
        repository={repository}
        repositoryRevision={1}
        service={remoteService}
        onUpdated={vi.fn()}
      />,
    );

    await screen.findByRole("button", { name: "Fetch origin" });
    fireEvent.click(screen.getByRole("button", { name: "Remote actions" }));
    fireEvent.click(screen.getByRole("button", { name: /Push 3/ }));

    expect(await screen.findByText("Pushed to origin.")).toBeDefined();
    expect(push).toHaveBeenCalledWith(repository.path, expect.any(Function));
  });
});
