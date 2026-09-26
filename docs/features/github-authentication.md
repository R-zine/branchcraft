# GitHub authentication feature

## User outcome

The user can sign in with GitHub in their normal browser, see the authenticated
account in Branchcraft, cancel a pending attempt, and sign out locally. Local
repository work remains available when GitHub is offline or OAuth is not
configured.

## Ownership

- `crates/auth` owns OAuth configuration, PKCE/state generation, the loopback
  listener, token exchange/refresh, session orchestration, and logout.
- `crates/storage` owns the opaque `SecretStore` contract and native keyring
  implementation.
- `crates/github` owns authenticated GitHub REST requests and public account
  and repository response mapping.
- `src-tauri/src/commands/auth.rs` is the thin IPC adapter.
- `frontend/src/features/auth` owns account-menu state and presentation.
- `frontend/src/services/authService.ts` is the only frontend auth invocation
  boundary.

## Contract

The frontend may receive `AuthStatus`, public `Account` and `GitHubRepository`
fields, and structured errors. It must never receive an authorization code,
access token, refresh token, PKCE verifier, or client secret. Remote Git and
GitHub clone commands obtain a short-lived `SecretString` only through the
backend `AuthService::access_token` method and pass it directly to the GitHub
API or libgit2 credential callback.

## Acceptance coverage

Offline backend tests use an injected browser, OAuth provider, GitHub service,
and in-memory secret store. They cover unconfigured builds, successful
loopback login, PKCE parameters, state rejection, current status, logout, and
cancellation. Frontend behavior tests cover sign-in, current-user display,
logout, and unconfigured UI without invoking Tauri or the network.
