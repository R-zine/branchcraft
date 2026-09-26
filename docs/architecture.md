# Architecture

## Scope

This document records the foundation selected on 2026-09-26 and the
Milestones 0–3 implementation. The application is an offline-capable local Git
client with bounded diffs, index staging, and local commit creation. OAuth,
secrets, network Git, and cloning are not part of the compiled application yet.

## Dependency baseline

The versions below were checked against their primary project documentation and
registries before implementation.

| Technology | Selected baseline | Rationale |
| --- | --- | --- |
| Rust | Stable 1.98.1; edition 2024; project MSRV 1.85 | Current stable toolchain, while 1.85 is the lowest edition-2024/keyring-4-compatible baseline. CI uses stable. |
| Tauri | 2.11.6 (`tauri`), 2.11.1 JS API | Current maintained Tauri 2 line; WebKitGTK gives a native Fedora/Wayland shell without Electron. |
| Tauri dialog | 2.7.2 | Official native dialog plugin with Linux directory-picker support. |
| React | 19.3.0 | Current stable React release. The UI uses only stable client APIs. |
| TypeScript | 6.0.3, strict mode | TypeScript 7.0.2 was inspected as the current release, but the maintained TypeScript ESLint 8.70 parser supports `<6.1`. The newest supported stable line avoids forcing an unsupported lint stack. |
| Vite | 8.3.1 | Current stable lightweight frontend toolchain. |
| git2 | 0.21.0 | Current maintained safe Rust bindings to libgit2, matching the requested initial implementation. |
| Monaco Editor | 0.57.0 | Current stable editor line; isolated behind a feature adapter and lazy-loaded only for previewable text diffs. |
| keyring | 4.2.0, deferred | Maintained Secret Service-compatible option for Fedora. It will be introduced with the storage/auth milestones, not linked before secrets exist. |

Sources: [Rust release notes](https://doc.rust-lang.org/releases.html),
[Tauri releases](https://v2.tauri.app/release/),
[Tauri dialog](https://v2.tauri.app/plugin/dialog/),
[React versions](https://react.dev/versions),
[TypeScript](https://www.typescriptlang.org/),
[Monaco package](https://www.npmjs.com/package/monaco-editor),
[git2 documentation](https://docs.rs/git2/latest/git2/), and
[keyring documentation](https://docs.rs/keyring/latest/keyring/).

The root `rust-toolchain.toml` follows stable instead of pinning 1.98.1 so clean
Fedora development environments receive Rust security and correctness updates.
Cargo/npm manifests constrain application dependencies and CI makes upgrades
visible.

TypeScript 7 should be adopted once TypeScript ESLint publishes compatible
parser support and the full frontend check suite passes without peer overrides.

## Runtime flow

```text
RepositoryHeader / WelcomeView
        |
        v
useRepository (feature-local state)
        |
        v
RepositoryService (typed frontend boundary)
        |
        v
Tauri repository command (thin async adapter)
        |
        v
GitService trait
        |
        v
LibGitService (git2/libgit2)
```

The native dialog returns a path to the frontend. The backend canonicalizes and
validates it before opening the directory. Repository inspection runs on a
blocking worker so filesystem/libgit2 work cannot stall the webview event loop.
Only structured domain results or user-safe `GitError` values cross IPC.

The diff path extends the same boundary without coupling the repository hook to
Monaco:

```text
selected FileStatus
        |
        v
useFileDiff -> DiffService -> Tauri diff command
        |                          |
        |                          v
        |                    GitService::diff
        |                          |
        v                          v
DiffViewer -> Monaco adapter    libgit2 + bounded file reads
```

Monaco is a lazy frontend chunk. Binary, oversized, and unsupported files do
not load it and receive dedicated fallback UI.

Staging and commits follow the same typed path. Mutation commands return a new
`RepositorySnapshot`, which replaces repository state and invalidates the
selected diff without introducing a global store. Index and commit work is
performed by libgit2 on blocking workers; the frontend never constructs Git
commands or edits `.git` directly.

## Rust workspace ownership

### `branchcraft-app-core`

Owns serializable domain models: repository identity, branch state, per-file
index/worktree status, structured file diffs, commit results, and safe error
categories. It has no dependency on Git, Tauri, Monaco, or a platform secret
store.

### `branchcraft-git`

Owns `GitService` and the libgit2 implementation. It opens a canonical path,
detects normal/detached/unborn HEAD states, preserves staged and unstaged status
independently, generates bounded `HEAD`-to-worktree file diffs, updates the
index, and creates commits from the index. It has no knowledge of IPC or UI.

### `branchcraft-test-support`

Builds real temporary Git repositories with deterministic local identities.
Network access and user-level Git configuration are never required by tests.

### `branchcraft` (`src-tauri`)

Owns the desktop process. Commands contain no Git implementation logic. The
managed `AppState` holds the Git trait object, making the boundary replaceable
and testable. The native dialog permission is explicitly limited to opening.

## Frontend ownership

- `features/repositories` owns repository loading and header/welcome UI.
- `features/changes` owns file status presentation, selection, and staging
  state.
- `features/commit` owns commit form state and validation.
- `features/diff` owns diff loading, fallback states, responsive layout, and the
  Monaco adapter.
- Files under `services` are the only raw `invoke()` boundaries, including the
  repository, diff, staging, and commit services.
- `types/repository.ts` and `types/diff.ts` mirror the serde IPC contracts.
- `components` contains presentation-only primitives, not domain behavior.

There is no global state object. The selected repository and file currently
live in the repository feature hook; unrelated future features should add their
own state rather than expanding a universal store.

## Security boundaries

- Frontend paths are untrusted. Rust rejects empty/non-directory paths and
  canonicalizes before passing them to libgit2.
- There is no shell execution.
- libgit2 opens exactly the selected directory; it does not search arbitrary
  parents.
- Diff paths must be ordinary relative paths. The backend rejects traversal and
  prevents resolved working-tree files from escaping the repository root.
- Diff content is capped at 2 MiB per side and binary/unsupported content never
  crosses IPC.
- Staging paths use the same normal-relative-path validation as diffs. Index
  mutations never invoke a shell or alter working-tree content while unstaging.
- Commits use the effective repository Git configuration for author/committer
  identity. Branchcraft never invents or rewrites identity settings.
- Raw libgit2 errors are mapped to structured categories instead of being shown
  or logged verbatim.
- No credential system is active in Milestones 0–3. When introduced, the auth
  crate will receive only a `SecretStore` abstraction backed by Secret Service;
  access tokens will never enter frontend models or Git remote URLs.

## Planned crates

`crates/auth`, `crates/github`, and `crates/storage` will be added only when
their milestones begin. Avoiding empty placeholder crates keeps every workspace
member useful and tested. Their contracts and ownership are already reserved:

- `auth`: OAuth state machine and account/session abstraction.
- `github`: GitHub HTTP API only, never local Git operations.
- `storage`: settings plus a Secret Service abstraction; secret values stay
  opaque outside the crate.

