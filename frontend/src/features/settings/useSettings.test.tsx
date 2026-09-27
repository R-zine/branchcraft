import { act, renderHook, waitFor } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";
import type { SettingsService } from "../../services/settingsService";
import { useSettings } from "./useSettings";

const originalMatchMedia = Object.getOwnPropertyDescriptor(window, "matchMedia");

afterEach(() => {
  if (originalMatchMedia === undefined) {
    Reflect.deleteProperty(window, "matchMedia");
  } else {
    Object.defineProperty(window, "matchMedia", originalMatchMedia);
  }
});

function service(): SettingsService {
  return {
    getSettings: vi.fn().mockResolvedValue({
      theme: "system",
      palette: "branchcraft",
      recentRepositories: [],
    }),
    setTheme: vi.fn(),
    setPalette: vi.fn(),
    removeRecentRepository: vi.fn(),
    clearRecentRepositories: vi.fn(),
    resetSettings: vi.fn(),
    applyWindowTheme: vi.fn().mockResolvedValue(undefined),
  };
}

describe("settings theme", () => {
  it("tracks changes to the system color scheme", async () => {
    let matches = true;
    const listeners = new Set<() => void>();
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn().mockImplementation(() => ({
        get matches() {
          return matches;
        },
        addEventListener: (_event: string, listener: () => void) => listeners.add(listener),
        removeEventListener: (_event: string, listener: () => void) => listeners.delete(listener),
      })),
    });

    const settingsService = service();
    const { result } = renderHook(() => useSettings(settingsService));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.resolvedTheme).toBe("light");
    expect(document.documentElement.dataset.theme).toBe("light");
    expect(document.documentElement.dataset.palette).toBe("branchcraft");

    act(() => {
      matches = false;
      listeners.forEach((listener) => listener());
    });
    expect(result.current.resolvedTheme).toBe("dark");
    expect(document.documentElement.dataset.theme).toBe("dark");
  });

  it("applies a persisted color palette", async () => {
    const setPalette = vi
      .fn<SettingsService["setPalette"]>()
      .mockResolvedValue({
        theme: "system",
        palette: "monochrome",
        recentRepositories: [],
      });
    const settingsService = { ...service(), setPalette };
    const { result } = renderHook(() => useSettings(settingsService));
    await waitFor(() => expect(result.current.loading).toBe(false));

    await act(async () => result.current.setPalette("monochrome"));

    expect(setPalette).toHaveBeenCalledWith("monochrome");
    expect(result.current.settings.palette).toBe("monochrome");
    expect(document.documentElement.dataset.palette).toBe("monochrome");
  });
});
