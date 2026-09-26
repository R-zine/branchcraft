import { useEffect, useRef, useState } from "react";
import type { AuthService } from "../../services/authService";
import { useAuth } from "./useAuth";

interface AccountMenuProps {
  service: AuthService;
}

export function AccountMenu({ service }: AccountMenuProps) {
  const [open, setOpen] = useState(false);
  const root = useRef<HTMLDivElement>(null);
  const auth = useAuth(service);
  const account = auth.status?.state === "signedIn" ? auth.status.account : null;

  useEffect(() => {
    if (!open) {
      return undefined;
    }
    const closeOnPointer = (event: PointerEvent) => {
      if (root.current?.contains(event.target as Node) === false) {
        setOpen(false);
      }
    };
    const closeOnEscape = (event: KeyboardEvent) => {
      if (event.key === "Escape" && !auth.signingIn) {
        setOpen(false);
      }
    };
    document.addEventListener("pointerdown", closeOnPointer);
    document.addEventListener("keydown", closeOnEscape);
    return () => {
      document.removeEventListener("pointerdown", closeOnPointer);
      document.removeEventListener("keydown", closeOnEscape);
    };
  }, [auth.signingIn, open]);

  const triggerLabel = account === null ? "GitHub account" : `GitHub account ${account.login}`;

  return (
    <div className="account-control" ref={root}>
      <button
        className="account-trigger"
        type="button"
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={triggerLabel}
        onClick={() => setOpen((current) => !current)}
      >
        <span className="account-avatar" aria-hidden="true">
          {account?.login.slice(0, 1).toUpperCase() ?? "GH"}
        </span>
        <span>{account === null ? "GitHub" : `@${account.login}`}</span>
      </button>

      {open && (
        <div className="account-popover" role="dialog" aria-label="GitHub account">
          {auth.loading ? (
            <div className="account-state" role="status">
              <span className="loading-spinner" /> Checking GitHub session…
            </div>
          ) : account !== null ? (
            <div className="account-profile">
              <span className="account-avatar large" aria-hidden="true">
                {account.login.slice(0, 1).toUpperCase()}
              </span>
              <div>
                <strong>{account.name ?? account.login}</strong>
                <span>@{account.login}</span>
              </div>
              <button
                type="button"
                disabled={auth.signingOut}
                onClick={() => void auth.logout()}
              >
                {auth.signingOut ? "Signing out…" : "Sign out"}
              </button>
            </div>
          ) : auth.status?.state === "unconfigured" ? (
            <div className="account-state">
              <strong>GitHub sign-in unavailable</strong>
              <span>This build has no GitHub OAuth App configuration.</span>
            </div>
          ) : auth.signingIn ? (
            <div className="account-state">
              <span className="loading-spinner" />
              <strong>Complete sign-in in your browser</strong>
              <span>Branchcraft is waiting on a private loopback callback.</span>
              <button type="button" onClick={() => void auth.cancelLogin()}>
                Cancel
              </button>
            </div>
          ) : (
            <div className="account-state">
              <strong>Connect GitHub</strong>
              <span>
                Authorize repository access in your browser. Credentials stay in
                the system keyring.
              </span>
              <button type="button" onClick={() => void auth.login()}>
                Sign in with GitHub
              </button>
            </div>
          )}

          {auth.error !== null && (
            <div className="account-error" role="alert">
              <strong>{auth.error.summary}</strong>
              {auth.error.detail !== null && <span>{auth.error.detail}</span>}
              <div>
                <button type="button" onClick={auth.dismissError}>
                  Dismiss
                </button>
                {!auth.signingIn && (
                  <button type="button" onClick={auth.reload}>
                    Retry status
                  </button>
                )}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
