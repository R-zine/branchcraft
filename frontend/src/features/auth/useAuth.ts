import { useCallback, useEffect, useRef, useState } from "react";
import type { AuthService } from "../../services/authService";
import {
  isAuthError,
  type AuthError,
  type AuthStatus,
} from "../../types/auth";

interface AuthState {
  status: AuthStatus | null;
  loading: boolean;
  signingIn: boolean;
  signingOut: boolean;
  error: AuthError | null;
  login: () => Promise<void>;
  cancelLogin: () => Promise<void>;
  logout: () => Promise<void>;
  reload: () => void;
  dismissError: () => void;
}

const unknownError: AuthError = {
  kind: "unknown",
  summary: "Branchcraft could not update your GitHub session.",
  detail: "Try again in a moment.",
};

export function useAuth(service: AuthService): AuthState {
  const [status, setStatus] = useState<AuthStatus | null>(null);
  const [loading, setLoading] = useState(true);
  const [signingIn, setSigningIn] = useState(false);
  const [signingOut, setSigningOut] = useState(false);
  const [error, setError] = useState<AuthError | null>(null);
  const [reloadToken, setReloadToken] = useState(0);
  const operationId = useRef(0);

  useEffect(() => {
    const currentOperation = operationId.current + 1;
    operationId.current = currentOperation;
    let active = true;
    setLoading(true);
    setError(null);

    void service
      .getStatus()
      .then((nextStatus) => {
        if (active && operationId.current === currentOperation) {
          setStatus(nextStatus);
        }
      })
      .catch((cause: unknown) => {
        if (active && operationId.current === currentOperation) {
          setError(isAuthError(cause) ? cause : unknownError);
        }
      })
      .finally(() => {
        if (active && operationId.current === currentOperation) {
          setLoading(false);
        }
      });
    return () => {
      active = false;
    };
  }, [reloadToken, service]);

  const login = useCallback(async () => {
    if (signingIn) {
      return;
    }
    const currentOperation = operationId.current + 1;
    operationId.current = currentOperation;
    setSigningIn(true);
    setError(null);
    try {
      const account = await service.login();
      if (operationId.current === currentOperation) {
        setStatus({ state: "signedIn", account });
      }
    } catch (cause: unknown) {
      if (operationId.current === currentOperation) {
        if (isAuthError(cause) && cause.kind === "cancelled") {
          setStatus({ state: "signedOut" });
        } else {
          setError(isAuthError(cause) ? cause : unknownError);
        }
      }
    } finally {
      if (operationId.current === currentOperation) {
        setSigningIn(false);
      }
    }
  }, [service, signingIn]);

  const cancelLogin = useCallback(async () => {
    try {
      await service.cancelLogin();
    } catch (cause: unknown) {
      setError(isAuthError(cause) ? cause : unknownError);
    }
  }, [service]);

  const logout = useCallback(async () => {
    if (signingOut) {
      return;
    }
    const currentOperation = operationId.current + 1;
    operationId.current = currentOperation;
    setSigningOut(true);
    setError(null);
    try {
      await service.logout();
      if (operationId.current === currentOperation) {
        setStatus({ state: "signedOut" });
      }
    } catch (cause: unknown) {
      if (operationId.current === currentOperation) {
        setError(isAuthError(cause) ? cause : unknownError);
      }
    } finally {
      if (operationId.current === currentOperation) {
        setSigningOut(false);
      }
    }
  }, [service, signingOut]);

  return {
    status,
    loading,
    signingIn,
    signingOut,
    error,
    login,
    cancelLogin,
    logout,
    reload: () => setReloadToken((current) => current + 1),
    dismissError: () => setError(null),
  };
}
