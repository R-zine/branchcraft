import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import type { AuthService } from "../services/authService";
import type { DiffService } from "../services/diffService";
import type { RepositoryService } from "../services/repositoryService";
import type { SettingsService } from "../services/settingsService";
import type { StagingService } from "../services/stagingService";
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

function authService(): AuthService {
  return {
    getStatus: vi.fn().mockResolvedValue({ state: "signedOut" }),
    login: vi.fn(),
    cancelLogin: vi.fn(),
    logout: vi.fn(),
  };
}

function settingsService(
  recentRepositories: Awaited<ReturnType<SettingsService["getSettings"]>>["recentRepositories"] = [],
) {
  const getSettings = vi.fn<SettingsService["getSettings"]>().mockResolvedValue({
    theme: "system",
    palette: "branchcraft",
    recentRepositories,
  });
  const setTheme = vi.fn<SettingsService["setTheme"]>().mockImplementation((theme) =>
    Promise.resolve({ theme, palette: "branchcraft", recentRepositories }),
  );
  const setPalette = vi.fn<SettingsService["setPalette"]>().mockImplementation((palette) =>
    Promise.resolve({ theme: "system", palette, recentRepositories }),
  );
  return {
    service: {
      getSettings,
      setTheme,
      setPalette,
      removeRecentRepository: vi.fn(),
      clearRecentRepositories: vi.fn(),
      resetSettings: vi.fn(),
      applyWindowTheme: vi.fn().mockResolvedValue(undefined),
    } satisfies SettingsService,
    getSettings,
    setTheme,
    setPalette,
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

function stagingService() {
  const stagedRepository: RepositorySnapshot = {
    ...repository,
    files: repository.files.map((file) => ({
      ...file,
      staged: "modified" as const,
      unstaged: null,
    })),
  };
  const stageFile = vi
    .fn<StagingService["stageFile"]>()
    .mockResolvedValue(stagedRepository);
  return {
    service: {
      stageFile,
      unstageFile: vi.fn(),
      stageAll: vi.fn(),
      unstageAll: vi.fn(),
    } satisfies StagingService,
    stageFile,
  };
}

describe("Branchcraft app", () => {
  it("opens a selected repository and renders its status", async () => {
    const { repositoryService, openRepository } = service();
    const settings = settingsService();
    render(
      <App
        authService={authService()}
        repositoryService={repositoryService}
        diffService={diffService()}
        settingsService={settings.service}
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
    const settings = settingsService();
    chooseDirectory.mockResolvedValue(null);
    render(
      <App
        authService={authService()}
        repositoryService={repositoryService}
        diffService={diffService()}
        settingsService={settings.service}
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

  it("applies the refreshed snapshot after staging a file", async () => {
    const { repositoryService } = service();
    const staging = stagingService();
    const settings = settingsService();
    render(
      <App
        authService={authService()}
        repositoryService={repositoryService}
        diffService={diffService()}
        stagingService={staging.service}
        settingsService={settings.service}
      />,
    );

    fireEvent.click(
      screen.getByRole("button", { name: /^open repository$/i }),
    );
    const checkbox = await screen.findByRole("checkbox", {
      name: "Stage src/main.rs",
    });
    fireEvent.click(checkbox);

    expect(
      await screen.findByRole("checkbox", { name: "Unstage src/main.rs" }),
    ).toBeDefined();
    expect(staging.stageFile).toHaveBeenCalledWith(
      "/work/branchcraft",
      "src/main.rs",
    );
  });

  it("opens settings from the keyboard and persists appearance choices", async () => {
    const { repositoryService } = service();
    const settings = settingsService();
    render(
      <App
        authService={authService()}
        repositoryService={repositoryService}
        diffService={diffService()}
        settingsService={settings.service}
      />,
    );

    fireEvent.keyDown(window, { key: ",", ctrlKey: true });
    expect(await screen.findByRole("dialog", { name: "Settings" })).toBeDefined();

    fireEvent.click(screen.getByRole("radio", { name: "Light" }));
    await waitFor(() => expect(settings.setTheme).toHaveBeenCalledWith("light"));
    expect(document.documentElement.dataset.theme).toBe("light");

    fireEvent.click(screen.getByRole("radio", { name: "Monochrome" }));
    await waitFor(() =>
      expect(settings.setPalette).toHaveBeenCalledWith("monochrome"),
    );
    expect(document.documentElement.dataset.palette).toBe("monochrome");

    fireEvent.keyDown(window, { key: "Escape" });
    expect(screen.queryByRole("dialog", { name: "Settings" })).toBeNull();
  });

  it("opens an available recent repository without showing the picker", async () => {
    const { repositoryService, chooseDirectory, openRepository } = service();
    const settings = settingsService([
      {
        name: "branchcraft",
        path: "/work/branchcraft",
        available: true,
      },
    ]);
    render(
      <App
        authService={authService()}
        repositoryService={repositoryService}
        diffService={diffService()}
        settingsService={settings.service}
      />,
    );

    fireEvent.click(await screen.findByTitle("/work/branchcraft"));
    await waitFor(() => expect(openRepository).toHaveBeenCalledWith("/work/branchcraft"));
    expect(chooseDirectory).not.toHaveBeenCalled();
  });
});

