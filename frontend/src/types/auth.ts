export interface Account {
  id: number;
  login: string;
  name: string | null;
  avatarUrl: string;
  profileUrl: string;
}

export type AuthStatus =
  | { state: "unconfigured" }
  | { state: "signedOut" }
  | { state: "signedIn"; account: Account };

export type AuthErrorKind =
  | "notConfigured"
  | "notAuthenticated"
  | "alreadyInProgress"
  | "listenerUnavailable"
  | "browserUnavailable"
  | "cancelled"
  | "stateMismatch"
  | "authorizationDenied"
  | "tokenExchange"
  | "network"
  | "authenticationExpired"
  | "credentialStore"
  | "invalidResponse"
  | "unknown";

export interface AuthError {
  kind: AuthErrorKind;
  summary: string;
  detail: string | null;
}

export function isAuthError(value: unknown): value is AuthError {
  return (
    typeof value === "object" &&
    value !== null &&
    "kind" in value &&
    "summary" in value
  );
}
