# Architecture

## Scope

This document records the foundation selected on 2026-09-26 and the
Milestones 0–8 implementation. Branchcraft is an offline-capable local Git
client with bounded diffs and history, staging and commits, safe local branch
management, browser-based GitHub authentication, and asynchronous authenticated
fetch/pull/push. It also browses the signed-in user's accessible GitHub
repositories and clones either a selected repository or a validated HTTPS URL.
It persists non-secret appearance and recent-repository settings, follows the
desktop theme, exposes application shortcuts, and includes Linux release
metadata and packaging workflows.

## Dependency baseline

| Technology | Selected baseline | Rationale |
| --- | --- | --- |
| Rust | Stable 1.98.1; edition 2024; project MSRV 1.85 | Stable development while retaining the lowest edition-2024/keyring-compatible baseline. |
| Tauri | 2.11.6 (`tauri`), 2.11.1 JS API | Native Fedora/Wayland desktop shell through WebKitGTK. |
| Tauri dialog/opener | 2.7.3 / 2.5.5 | Official native directory picker and Rust-side system-browser launch. |
| React | 19.3.0 | Stable client UI APIs. |
| TypeScript | 6.0.3, strict mode | Newest stable line supported by the selected TypeScript ESLint parser. |
| Vite | 8.3.1 | Lightweight frontend build and development server. |
| git2 | 0.21.0 | Safe Rust bindings to libgit2 behind `GitService`. |
| Monaco Editor | 0.57.0 | Lazy-loaded, feature-isolated text diff renderer. |
| reqwest | 0.13.5 with rustls | GitHub OAuth and REST client with bounded timeouts. |
| keyring | 4.2.0 | Native credential-store abstraction; Linux uses Secret Service over zbus. |
| secrecy | 0.10.3 | Redacted secret wrappers for credentials and one-time OAuth values. |

Primary references: [Rust releases](https://doc.rust-lang.org/releases.html),
[Tauri releases](https://v2.tauri.app/release/),
[Tauri plugins](https://v2.tauri.app/plugin/),
[React versions](https://react.dev/versions),
[git2](https://docs.rs/git2/latest/git2/),
[reqwest](https://docs.rs/reqwest/latest/reqwest/), and
[keyring](https://docs.rs/keyring/latest/keyring/).

The root `rust-toolchain.toml` follows stable so clean Fedora environments
receive Rust security and correctness updates. Cargo/npm manifests constrain
application dependencies and CI makes upgrades visible.

## Runtime flows

Local repository features share a typed, one-way boundary:

```text
React feature -> typed frontend service -> thin Tauri command
                                           |
                                           v
                                      GitService
                                           |
                                           v
                                  LibGitService / libgit2
```

The native dialog returns a path to React. Rust canonicalizes and validates it
before opening the exact directory. Filesystem and libgit2 work runs on blocking
workers. Only domain results or user-safe `GitError` values cross IPC.

Selected-file diff loading remains independent of repository state and Monaco:

```text
FileStatus -> useFileDiff -> DiffService -> GitService::diff
                  |
                  v
             DiffViewer -> lazy Monaco adapter or bounded fallback
```

Binary, oversized, and unsupported files do not load Monaco. Mutation commands
return a fresh `RepositorySnapshot`, replacing feature state and invalidating
the selected diff without a global store. History is bounded to 200 commits.
Branch mutations return both the refreshed snapshot and branch list so the
header, working tree, diff, and branch picker remain synchronized.

GitHub authentication is independent of repository state:

```text
AccountMenu -> AuthService (frontend) -> auth Tauri command
                                         |
                                         v
                                BranchcraftAuth
                            /          |          \
                   system browser  GitHub API  SecretStore
                         |             |          |
                  127.0.0.1 PKCE    GET /user  OS keyring
```

The Tauri layer constructs services and opens the system browser, while the
OAuth state machine stays in `branchcraft-auth`. Only public `Account`,
`AuthStatus`, and safe `AuthError` values cross IPC.

Remote Git operations reuse those backend-only credentials:

```text
RemoteControls -> RemoteService -> async Tauri command -> blocking worker
                                                        /              \
                                             AuthService token     GitService
                                                                      |
                                                           libgit2 callbacks
                                                                      |
                                                       GitHub HTTPS or local remote
```

The command inspects the active remote before asking `AuthService` for a token.
Local remotes need no credential; only an exact, credential-free
`https://github.com` URL can receive the token through libgit2's credential
callback. Progress is streamed to React over a typed Tauri channel. The worker
returns a fresh repository snapshot and remote ahead/behind counts so all
feature state stays synchronized.

Repository discovery and cloning use the same one-way service boundaries:

```text
CloneView -> GitHubRepositoryService -> Tauri command -> GitHubService
                                                        |
                                                  GET /user/repos

CloneView -> CloneService -> native parent picker -> Tauri blocking worker
                                                        |
                                                   GitService / libgit2
```

The GitHub response is bounded to ten 100-item pages and contains only public
repository metadata. Cloning streams connection, transfer, checkout, and
completion progress, then returns the new repository snapshot so it opens in
the existing workspace immediately.

Settings stay separate from repository state and secret storage:

```text
SettingsPanel / WelcomeView -> SettingsService -> settings Tauri command
                                                    |
                                              SettingsStore
                                                    |
                                      app config/settings.json

desktop color-scheme -> useSettings -> CSS + Monaco + native Tauri window
```

Opening or cloning a repository records its canonical path on the blocking
worker. A settings-write failure is logged without failing the Git operation.
Recent availability is refreshed from disk at load time, and unavailable paths
remain removable from the settings UI.

## Rust workspace ownership

### `branchcraft-app-core`

Owns serializable, implementation-neutral domain models: repository identity,
branch state, file status, diffs, history and mutation results, public GitHub
account and repository data, clone progress, application settings, and
structured error/status types. It has no Tauri, GitHub HTTP, libgit2, or
platform-keyring dependency.

### `branchcraft-git`

Owns `GitService` and its libgit2 implementation: exact-directory repository
opening, normal/detached/unborn HEAD, separate index/worktree status, bounded
diffs, staging, commits, history, local branch operations, remote selection,
fetch, safe pull integration, push, and transport/progress error mapping. It
also owns clone-source validation, collision-safe destination derivation, and
clone progress. It has no IPC, UI, OAuth, or GitHub API knowledge; a caller may
provide a redacted backend secret to its credential callback.

### `branchcraft-auth`

Owns GitHub OAuth App configuration, per-attempt PKCE and CSRF state, the
ephemeral IPv4 loopback callback, token exchange/refresh, current-session
orchestration, cancellation, and logout. It depends on `GitHubService`,
`SecretStore`, and `BrowserLauncher` contracts and exposes credentials only as
backend `SecretString` values.

### `branchcraft-github`

Owns GitHub-specific HTTP behavior. It implements the versioned `GET /user`
request and the bounded, paginated `GET /user/repos` request, mapping only
public account and repository fields into application models. It has no local
Git or keyring responsibility.

### `branchcraft-storage`

Owns both persistence contracts while keeping their data physically separate.
`KeyringSecretStore` delegates opaque credentials to the native credential
store; on Linux this is Secret Service/GNOME Keyring. `JsonSettingsStore`
serializes bounded, non-secret preferences below Tauri's application-config
directory with size checks, serialized updates, and safe corruption errors.

### `branchcraft-test-support`

Builds real temporary Git repositories with deterministic local identities and
provides an in-memory `SecretStore` for offline authentication tests. Tests do
not require network access or user-level Git configuration.

### `branchcraft` (`src-tauri`)

Owns the desktop process and dependency assembly. Commands contain no Git,
OAuth, GitHub HTTP, or storage implementation logic. `AppState` holds trait
objects for Git, authentication, GitHub HTTP, and settings. Commands obtain
credentials only in the backend and move blocking Git/settings work off the
async runtime. Browser launch happens from Rust, so no opener capability is
granted to webview JavaScript; native permissions remain limited to opening a
dialog and synchronizing the main window theme.

## Frontend ownership

- `features/repositories` owns repository loading and header/welcome UI.
- `features/changes` owns file status, selection, and staging state.
- `features/commit` owns commit form state and validation.
- `features/diff` owns diff loading, fallbacks, responsive presentation, and
  the Monaco adapter.
- `features/history` owns bounded history loading and presentation.
- `features/branches` owns search, creation, switching, and safe deletion.
- `features/auth` owns account status, browser sign-in/cancellation state,
  logout, and recoverable error UI.
- `features/remotes` owns remote discovery, streamed operation progress,
  ahead/behind state, fetch/pull/push actions, and recovery UI.
- `features/cloning` owns GitHub repository browsing, client-side search and
  filtering, clone-by-selection and clone-by-URL, progress, and recovery UI.
- `features/settings` owns appearance, recents management, About content,
  application shortcuts, system-theme observation, and recovery UI.
- `services` is the only location for raw Tauri `invoke()` calls.
- `types` mirrors serde IPC contracts; credentials have no frontend type.
- `components` contains presentation-only primitives.

There is no global state object. Features own their state and communicate using
explicit result contracts rather than expanding a universal store.

## Security boundaries

- Frontend paths are untrusted. Rust rejects invalid paths and prevents diff and
  staging paths from escaping the selected repository.
- There is no shell execution. Git operations use libgit2.
- Diff content is capped at 2 MiB per side; unsupported and binary content never
  crosses IPC.
- Checkout uses libgit2's safe strategy. Branch deletion rejects the current or
  unmerged branch; there is no force-delete path.
- OAuth state and PKCE verifiers use 256 random bits from an OS-seeded CSPRNG per
  attempt. State is compared in constant time.
- The callback listens only on `127.0.0.1` with an ephemeral port, accepts the
  exact callback path, bounds request size/time, and never reflects parameters
  into its browser response.
- Authorization codes and tokens use secret wrappers and never enter frontend
  models, logs, application configuration, process arguments, or Git URLs.
- OAuth sessions exist only in the platform keyring. The in-memory JSON envelope
  is passed directly to `SecretStore`; it is never written as a file.
- The ordinary JSON settings file contains only theme choice and repository
  display names/paths. It never contains OAuth values or remote credentials.
- HTTP redirects are disabled for token exchange. HTTP failures are mapped to
  safe categories without surfacing response bodies or credentials.
- Remote Git follows no redirects. OAuth credentials are supplied only through
  libgit2 callbacks for exact GitHub HTTPS URLs; SSH, credential-bearing URLs,
  non-GitHub hosts, query strings, and fragments are rejected before network
  access. Tokens are never written to remote URLs or Git credential
  configuration.
- Clone URLs must be credential-free HTTPS URLs without a query or fragment.
  Only exact GitHub HTTPS callbacks can receive an OAuth token; other HTTPS
  hosts clone anonymously, and redirects are disabled. Rust canonicalizes the
  selected parent directory, derives a single normal child name, and refuses
  to overwrite an existing destination.
- Pull refuses a dirty worktree. Diverged histories are merged in memory first;
  a conflict leaves `HEAD`, the index, and working files unchanged. Push
  rejection and non-fast-forward results are explicit errors.
- Production logging accepts only Branchcraft-targeted records, rotates at
  1 MiB, and logs operation categories rather than repository paths, server
  payloads, or credentials.
