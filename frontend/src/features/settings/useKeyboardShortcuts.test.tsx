import { fireEvent, render } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { useKeyboardShortcuts } from "./useKeyboardShortcuts";

function Harness({
  settingsOpen,
  actions,
}: {
  settingsOpen: boolean;
  actions: ReturnType<typeof shortcutActions>;
}) {
  useKeyboardShortcuts({ ...actions, settingsOpen });
  return null;
}

function shortcutActions() {
  return {
    openRepository: vi.fn(),
    cloneRepository: vi.fn(),
    refreshRepository: vi.fn(),
    showChanges: vi.fn(),
    showHistory: vi.fn(),
    openSettings: vi.fn(),
    closeSettings: vi.fn(),
  };
}

describe("keyboard shortcuts", () => {
  it("maps repository, view, and settings shortcuts", () => {
    const actions = shortcutActions();
    render(<Harness settingsOpen={false} actions={actions} />);

    fireEvent.keyDown(window, { key: "o", ctrlKey: true });
    fireEvent.keyDown(window, { key: "O", ctrlKey: true, shiftKey: true });
    fireEvent.keyDown(window, { key: "r", ctrlKey: true });
    fireEvent.keyDown(window, { key: "1", ctrlKey: true });
    fireEvent.keyDown(window, { key: "2", ctrlKey: true });
    fireEvent.keyDown(window, { key: ",", ctrlKey: true });

    expect(actions.openRepository).toHaveBeenCalledOnce();
    expect(actions.cloneRepository).toHaveBeenCalledOnce();
    expect(actions.refreshRepository).toHaveBeenCalledOnce();
    expect(actions.showChanges).toHaveBeenCalledOnce();
    expect(actions.showHistory).toHaveBeenCalledOnce();
    expect(actions.openSettings).toHaveBeenCalledOnce();
  });

  it("closes settings with Escape and suppresses background shortcuts", () => {
    const actions = shortcutActions();
    render(<Harness settingsOpen actions={actions} />);

    fireEvent.keyDown(window, { key: "o", ctrlKey: true });
    fireEvent.keyDown(window, { key: "Escape" });

    expect(actions.openRepository).not.toHaveBeenCalled();
    expect(actions.closeSettings).toHaveBeenCalledOnce();
  });
});
