# Cloning and GitHub discovery

## User outcome

A signed-in user can browse the GitHub repositories available to their account,
search by repository name, description, or language, and filter by visibility
or archived status. They can clone a listed repository or enter a public HTTPS
repository URL directly. A native directory picker chooses the destination
parent, progress remains visible during network and checkout work, and a
successful clone opens immediately in the normal repository workspace.

The GitHub list is bounded to the 1,000 most recently pushed repositories. A
truncation notice is shown when the backend reaches that safety limit. Failure
to load the authenticated list does not disable clone-by-URL.

## Ownership

- `frontend/src/features/cloning` owns discovery, search/filter state, clone
  forms, progress, and user-facing recovery.
- `frontend/src/services/githubRepositoryService.ts` and
  `frontend/src/services/cloneService.ts` are the typed IPC boundaries.
- `src-tauri/src/commands/github.rs` obtains a backend access token and delegates
  repository listing to `GitHubService`.
- `src-tauri/src/commands/cloning.rs` selects credentials by source type and
  runs libgit2 cloning on a blocking worker.
- `crates/github` owns bounded GitHub REST pagination and response mapping.
- `crates/git` owns clone-source validation, safe destination derivation,
  libgit2 callbacks, progress, and structured Git errors.
- `crates/app-core` owns the serializable repository and clone-progress models.

## Security contract

Clone URLs cannot contain embedded credentials, query strings, or fragments.
HTTP and SSH sources are rejected. Only exact `https://github.com` sources may
receive the OAuth token, and libgit2 repeats that host check inside its
credential callback. Other HTTPS hosts clone anonymously. HTTP and Git
redirects are disabled so credentials cannot be forwarded to another host.

The destination parent must already exist and is canonicalized by Rust. The
repository directory name is derived as one normal path component, and cloning
refuses an existing destination instead of overwriting it. A failed clone may
leave a partial directory, which the error describes rather than deleting user
data automatically.

## Acceptance coverage

Offline Rust tests cover authenticated paginated GitHub repository mapping,
unsafe clone-source rejection, local fixture cloning with streamed progress,
and destination collisions. Frontend behavior tests cover search/filtering,
listed-repository cloning, URL cloning, native destination selection, progress,
opening the returned snapshot, and keeping URL cloning available when GitHub
discovery fails.

References:

- [GitHub REST repositories API](https://docs.github.com/en/rest/repos/repos)
- [libgit2 authentication guide](https://libgit2.org/docs/guides/authentication/)
