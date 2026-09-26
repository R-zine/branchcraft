# Remotes (Milestone 6)

## User outcome

The repository header discovers the current branch's upstream remote, falling
back to `origin` and then the first configured remote. It shows ahead/behind
counts and provides fetch, pull, and push without blocking the desktop UI.
Operation phases are visible while Branchcraft connects, transfers objects,
updates references, or integrates changes. Completion and failure states remain
in the remote panel until dismissed or retried.

Fetch updates remote-tracking references. Pull fetches first, reports an
up-to-date branch, fast-forwards when possible, or creates a normal two-parent
merge commit for a clean non-conflicting divergence. A dirty worktree stops
integration, and an in-memory merge conflict leaves `HEAD`, the index, and all
working files untouched. Push publishes the current branch and reports
non-fast-forward and server rejection separately.

## Authentication and transport policy

Production authentication supports credential-free GitHub.com HTTPS remote
URLs. The Tauri command obtains a backend-only token from `AuthService`; libgit2
receives it through a credential callback with `x-access-token` as the username.
The token never enters React, a Git URL, repository configuration, process
arguments, or logs.

Redirects are disabled. SSH URLs, embedded credentials, non-GitHub network
hosts, query strings, and fragments are rejected with recovery guidance. Local
absolute paths and hostless `file:` remotes remain available without
authentication, which also keeps ordinary tests entirely offline.

References:

- [GitHub: managing tokens for HTTPS Git](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/managing-your-personal-access-tokens)
- [GitHub: remote repository URL formats](https://docs.github.com/en/get-started/git-basics/about-remote-repositories)
- [libgit2 authentication callbacks](https://libgit2.org/docs/guides/authentication/)

## Ownership

- `branchcraft-app-core` owns serializable remote, progress, outcome, and error
  contracts.
- `branchcraft-git` owns remote selection and every libgit2 operation.
- `src-tauri/src/commands/remotes.rs` obtains authentication, moves blocking
  work off the async runtime, and forwards typed progress events.
- `frontend/src/features/remotes` owns the header controls, feature state, and
  recovery presentation.
- `frontend/src/services/remoteService.ts` is the only remote-operation IPC
  boundary.

## Acceptance coverage

Offline libgit2 tests cover no remote, unsupported and credential-bearing URLs,
missing GitHub credentials, invalid local remotes, fetch plus fast-forward,
successful and non-fast-forward pushes, clean diverged merges, dirty pulls, and
conflicting pulls that preserve repository state.
Frontend tests cover no-remote guidance, operation progress/results, ahead-count
push behavior, and actionable authentication failures. Live network access is
not required by the test suite.
