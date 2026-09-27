export type ThemePreference = "system" | "light" | "dark";
export type ResolvedTheme = "light" | "dark";
export type ColorPalette = "branchcraft" | "ocean" | "forest" | "monochrome";

export interface RecentRepository {
  name: string;
  path: string;
  available: boolean;
}

export interface ApplicationSettings {
  theme: ThemePreference;
  palette: ColorPalette;
  recentRepositories: RecentRepository[];
}

export type SettingsErrorKind = "read" | "write" | "corrupt" | "unknown";

export interface SettingsError {
  kind: SettingsErrorKind;
  summary: string;
  detail: string | null;
}

export function isSettingsError(value: unknown): value is SettingsError {
  if (typeof value !== "object" || value === null) {
    return false;
  }
  const candidate = value as Partial<SettingsError>;
  return (
    typeof candidate.kind === "string" &&
    typeof candidate.summary === "string" &&
    (candidate.detail === null || typeof candidate.detail === "string")
  );
}
