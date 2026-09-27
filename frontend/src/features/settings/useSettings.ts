import {
  useCallback,
  useEffect,
  useLayoutEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import type { SettingsService } from "../../services/settingsService";
import {
  isSettingsError,
  type ApplicationSettings,
  type ColorPalette,
  type ResolvedTheme,
  type SettingsError,
  type ThemePreference,
} from "../../types/settings";

const defaults: ApplicationSettings = {
  theme: "system",
  palette: "branchcraft",
  recentRepositories: [],
};

const unknownError: SettingsError = {
  kind: "unknown",
  summary: "Branchcraft settings are temporarily unavailable.",
  detail: "Try again or reset settings to recover safe defaults.",
};

function preferredSystemTheme(): ResolvedTheme {
  if (
    typeof window.matchMedia === "function" &&
    window.matchMedia("(prefers-color-scheme: light)").matches
  ) {
    return "light";
  }
  return "dark";
}

export interface SettingsState {
  settings: ApplicationSettings;
  resolvedTheme: ResolvedTheme;
  loading: boolean;
  busy: boolean;
  error: SettingsError | null;
  setTheme: (theme: ThemePreference) => Promise<void>;
  setPalette: (palette: ColorPalette) => Promise<void>;
  removeRecentRepository: (path: string) => Promise<void>;
  clearRecentRepositories: () => Promise<void>;
  reset: () => Promise<void>;
  dismissError: () => void;
}

export function useSettings(service: SettingsService): SettingsState {
  const [settings, setSettings] = useState(defaults);
  const [systemTheme, setSystemTheme] = useState(preferredSystemTheme);
  const [loading, setLoading] = useState(true);
  const [busy, setBusy] = useState(false);
  const operationBusy = useRef(false);
  const [error, setError] = useState<SettingsError | null>(null);
  const resolvedTheme = useMemo<ResolvedTheme>(
    () => (settings.theme === "system" ? systemTheme : settings.theme),
    [settings.theme, systemTheme],
  );

  useEffect(() => {
    let active = true;
    void service
      .getSettings()
      .then((result) => {
        if (active) {
          setSettings(result);
        }
      })
      .catch((cause: unknown) => {
        if (active) {
          setError(isSettingsError(cause) ? cause : unknownError);
        }
      })
      .finally(() => {
        if (active) {
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [service]);

  useEffect(() => {
    if (typeof window.matchMedia !== "function") {
      return undefined;
    }
    const preference = window.matchMedia("(prefers-color-scheme: light)");
    const update = () => setSystemTheme(preference.matches ? "light" : "dark");
    preference.addEventListener("change", update);
    return () => preference.removeEventListener("change", update);
  }, []);

  useLayoutEffect(() => {
    document.documentElement.dataset.theme = resolvedTheme;
    document.documentElement.dataset.palette = settings.palette;
    document.documentElement.style.colorScheme = resolvedTheme;
  }, [resolvedTheme, settings.palette]);

  useEffect(() => {
    void service.applyWindowTheme(settings.theme).catch(() => undefined);
  }, [service, settings.theme]);

  const update = useCallback(
    async (operation: () => Promise<ApplicationSettings>) => {
      if (operationBusy.current) {
        return;
      }
      operationBusy.current = true;
      setBusy(true);
      setError(null);
      try {
        setSettings(await operation());
      } catch (cause: unknown) {
        setError(isSettingsError(cause) ? cause : unknownError);
      } finally {
        operationBusy.current = false;
        setBusy(false);
      }
    },
    [],
  );

  return {
    settings,
    resolvedTheme,
    loading,
    busy,
    error,
    setTheme: (theme) => update(() => service.setTheme(theme)),
    setPalette: (palette) => update(() => service.setPalette(palette)),
    removeRecentRepository: (path) =>
      update(() => service.removeRecentRepository(path)),
    clearRecentRepositories: () => update(() => service.clearRecentRepositories()),
    reset: () => update(() => service.resetSettings()),
    dismissError: () => setError(null),
  };
}
