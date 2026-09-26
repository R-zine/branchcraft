import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type {
  ApplicationSettings,
  ThemePreference,
} from "../types/settings";

export interface SettingsService {
  getSettings(): Promise<ApplicationSettings>;
  setTheme(theme: ThemePreference): Promise<ApplicationSettings>;
  removeRecentRepository(path: string): Promise<ApplicationSettings>;
  clearRecentRepositories(): Promise<ApplicationSettings>;
  resetSettings(): Promise<ApplicationSettings>;
  applyWindowTheme(theme: ThemePreference): Promise<void>;
}

export const tauriSettingsService: SettingsService = {
  getSettings() {
    return invoke<ApplicationSettings>("get_settings");
  },

  setTheme(theme) {
    return invoke<ApplicationSettings>("set_theme", {
      request: { theme },
    });
  },

  removeRecentRepository(path) {
    return invoke<ApplicationSettings>("remove_recent_repository", {
      request: { path },
    });
  },

  clearRecentRepositories() {
    return invoke<ApplicationSettings>("clear_recent_repositories");
  },

  resetSettings() {
    return invoke<ApplicationSettings>("reset_settings");
  },

  applyWindowTheme(theme) {
    return getCurrentWindow().setTheme(theme === "system" ? null : theme);
  },
};
