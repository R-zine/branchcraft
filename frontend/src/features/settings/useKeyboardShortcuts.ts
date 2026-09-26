import { useEffect } from "react";

interface KeyboardShortcutActions {
  openRepository: () => void;
  cloneRepository: () => void;
  refreshRepository: () => void;
  showChanges: () => void;
  showHistory: () => void;
  openSettings: () => void;
  closeSettings: () => void;
  settingsOpen: boolean;
}

export function useKeyboardShortcuts(actions: KeyboardShortcutActions) {
  const {
    openRepository,
    cloneRepository,
    refreshRepository,
    showChanges,
    showHistory,
    openSettings,
    closeSettings,
    settingsOpen,
  } = actions;
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.defaultPrevented || event.repeat) {
        return;
      }
      if (event.key === "Escape" && settingsOpen) {
        event.preventDefault();
        closeSettings();
        return;
      }
      if (settingsOpen) {
        return;
      }
      if (!(event.ctrlKey || event.metaKey) || event.altKey) {
        return;
      }

      const key = event.key.toLowerCase();
      if (key === "o") {
        event.preventDefault();
        if (event.shiftKey) {
          cloneRepository();
        } else {
          openRepository();
        }
      } else if (key === "r" && !event.shiftKey) {
        event.preventDefault();
        refreshRepository();
      } else if (key === "1" && !event.shiftKey) {
        event.preventDefault();
        showChanges();
      } else if (key === "2" && !event.shiftKey) {
        event.preventDefault();
        showHistory();
      } else if (key === "," && !event.shiftKey) {
        event.preventDefault();
        openSettings();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [
    cloneRepository,
    closeSettings,
    openRepository,
    openSettings,
    refreshRepository,
    settingsOpen,
    showChanges,
    showHistory,
  ]);
}
