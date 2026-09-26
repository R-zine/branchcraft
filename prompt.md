You are implementing **Branchcraft**, a production-quality desktop Git client for Fedora Linux, inspired by the workflow and usability of GitHub Desktop, but not intended to be a visual clone.

Repository name:

```text
branchcraft
```

Primary application identity:

```text
Product name: Branchcraft
Repository: branchcraft
Binary: branchcraft
Cargo workspace: branchcraft
Tauri productName: Branchcraft
Suggested Linux application ID: io.github.<username>.Branchcraft
```

The primary goal is a fast, polished, understandable Git GUI for a developer working mostly with their own GitHub repositories.

The application should be modular enough that multiple coding agents can work on independent features without frequently modifying the same files.

## 1. Product goals

Build Branchcraft as a native-installable Fedora desktop application with these characteristics:

- Rust backend.
- Tauri 2 desktop shell.
- TypeScript frontend.
- React frontend unless there is a strong technical reason to choose another framework.
- Monaco Editor for code/diff rendering.
- GitHub authentication through OAuth in the user's browser.
- No requirement for users to manually create SSH keys or personal access tokens.
- HTTPS Git authentication using the authenticated GitHub account.
- Tokens must be stored through the Linux Secret Service / GNOME Keyring, never in ordinary config files.
- Excellent dark-mode UI.
- Fast startup and low idle resource usage.
- The interface should prioritize clarity and discoverability over exposing every Git feature.

The UX target is approximately:

> "GitHub Desktop, designed specifically as a lightweight Fedora/Linux desktop application."

Do not reproduce GitHub Desktop's branding, assets, or visual design.

Branchcraft should develop its own visual identity.

---

# 2. Core architecture

Organize the application as a Cargo workspace with independent modules.

Preferred high-level structure:

```text
branchcraft/
├── Cargo.toml
├── crates/
│   ├── app-core/
│   ├── auth/
│   ├── git/
│   ├── github/
│   ├── storage/
│   └── test-support/
│
├── src-tauri/
│   ├── src/
│   │   ├── commands/
│   │   ├── events/
│   │   └── main.rs
│   └── tauri.conf.json
│
├── frontend/
│   ├── src/
│   │   ├── app/
│   │   ├── features/
│   │   ├── components/
│   │   ├── services/
│   │   ├── state/
│   │   └── types/
│   └── ...
│
├── docs/
│   ├── architecture.md
│   ├── auth.md
│   ├── development.md
│   └── features/
│
└── tests/
```

The Rust crates should have clean contracts and minimal coupling.

Suggested responsibilities:

### `app-core`

Shared domain models and interfaces.

Examples:

- RepositoryId
- RepositoryInfo
- BranchInfo
- CommitInfo
- FileStatus
- Diff
- DiffFile
- DiffHunk
- DiffLine
- GitError
- Account
- RemoteInfo

This crate should have very few dependencies.

### `auth`

Responsible for:

- GitHub OAuth authorization.
- PKCE.
- CSRF/state validation.
- Loopback callback listener.
- Token retrieval.
- Token persistence.
- Logout.
- Authentication status.

The rest of Branchcraft must not know how credentials are stored.

Expose an abstraction resembling:

```rust
trait AuthService {
    async fn login(&self) -> Result<Account, AuthError>;
    async fn logout(&self) -> Result<(), AuthError>;
    async fn current_account(&self) -> Result<Option<Account>, AuthError>;
    async fn access_token(&self) -> Result<SecretString, AuthError>;
}
```

Do not expose raw credentials to the frontend.

### `storage`

Responsible for:

- Linux Secret Service / GNOME Keyring access.
- Non-secret application preferences.
- Recently opened repositories.
- Window/application state where appropriate.

OAuth access tokens must never be stored in:

- JSON config
- SQLite plaintext
- logs
- frontend localStorage
- frontend sessionStorage
- Git remote URLs

Use the operating system's secret store.

### `git`

All Git functionality belongs here.

Use `git2`/libgit2 initially unless a specific operation demonstrably requires invoking the Git CLI.

Design the module behind an interface so implementation details can later change.

Conceptually:

```rust
trait GitService {
    fn open_repository(...);
    fn repository_status(...);
    fn diff(...);

    fn stage_file(...);
    fn unstage_file(...);
    fn stage_hunk(...);
    fn discard_file(...);

    fn commit(...);

    fn branches(...);
    fn create_branch(...);
    fn checkout_branch(...);
    fn delete_branch(...);

    async fn fetch(...);
    async fn pull(...);
    async fn push(...);

    async fn clone_repository(...);
}
```

Do not make frontend components execute Git commands directly.

Network authentication should be injected into Git operations by the Rust backend using credentials from `AuthService`.

Never construct authenticated URLs containing an OAuth token.

### `github`

Responsible only for GitHub-specific HTTP API operations.

Examples:

- authenticated user
- list repositories
- repository metadata
- default branch
- pull requests
- opening remote URLs
- remote repository discovery

Use `reqwest` or another mature Rust HTTP client.

Keep Git operations and GitHub API operations separate.

A repository must still work locally when offline.

---

# 3. GitHub authentication

Implement GitHub OAuth using an OAuth App, not a GitHub App.

Authentication flow:

```text
Branchcraft
   ↓
"Sign in with GitHub"
   ↓
Open system browser
   ↓
GitHub OAuth authorization
   ↓
loopback redirect to 127.0.0.1:<ephemeral port>
   ↓
validate state
   ↓
exchange authorization code
   ↓
retrieve authenticated GitHub user
   ↓
store token in GNOME Keyring / Secret Service
   ↓
authenticated Branchcraft session
```

Requirements:

- Authorization Code flow.
- PKCE where supported/recommended.
- Random cryptographically secure `state`.
- Bind callback listener only to loopback.
- Prefer a random available local port.
- Callback should show a simple browser response such as:
  "Authentication complete. You can return to Branchcraft."
- Handle cancellation cleanly.
- Handle port/listener failures.
- Handle OAuth errors explicitly.
- Never log authorization codes or access tokens.
- Never send OAuth credentials to the web frontend.

OAuth client ID can be public application configuration.

Do not embed a meaningful reusable client secret in the desktop binary and pretend it is confidential.

Research current GitHub OAuth requirements before implementing the flow and document any deviations from this specification necessitated by GitHub's current API.

---

# 4. Main application UX

Branchcraft should use a desktop-oriented layout rather than looking like a website inside a window.

Primary screen:

```text
┌───────────────────────────────────────────────────────────────────┐
│ Branchcraft                                                       │
│ Repository ▼             Current branch ▼          Fetch origin   │
├────────────────────┬──────────────────────────────────────────────┤
│ Changes / History  │                                              │
│                    │                                              │
│ [ ] src/main.rs    │            Diff viewer                       │
│ [x] README.md      │                                              │
│ [ ] Cargo.toml     │                                              │
│                    │                                              │
│                    │                                              │
├────────────────────┼──────────────────────────────────────────────┤
│ Summary            │                                              │
│ Description        │                                              │
│                    │                                              │
│ [ Commit ]         │                                              │
└────────────────────┴──────────────────────────────────────────────┘
```

Major application areas:

1. Repository selection
2. Current repository
3. Changes
4. Diff viewer
5. Commit form
6. Commit history
7. Branch management
8. Clone repository
9. Add existing local repository
10. Create repository
11. GitHub account
12. Settings

Use separate feature modules rather than a single large application component.

---

# 5. Frontend architecture

Structure features approximately like:

```text
frontend/src/features/
├── auth/
├── repositories/
├── changes/
├── diff/
├── commit/
├── history/
├── branches/
├── clone/
├── github/
└── settings/
```

Each feature should contain its own:

- UI components
- hooks
- frontend state
- Tauri command wrapper
- tests

Avoid a global state object containing the entire application.

Prefer feature-local state plus a small amount of application-wide state such as:

- selected repository
- authenticated account
- selected branch
- application theme

All interaction with Rust must go through typed service wrappers.

Do not scatter raw `invoke()` calls throughout React components.

Example:

```text
React component
      ↓
frontend GitService
      ↓
Tauri command
      ↓
Rust command adapter
      ↓
git crate
```

---

# 6. Diff viewer

The diff viewer is one of Branchcraft's most important features.

Use Monaco Editor.

Support both:

- unified/inline diff
- side-by-side diff

Start with side-by-side if implementing both at once increases complexity.

Requirements:

- syntax highlighting
- line numbers
- added/deleted line styling
- inline word-level differences
- large-file handling
- binary-file detection
- empty file support
- new file support
- deleted file support
- renamed file support
- whitespace-only changes
- horizontal scrolling
- synchronized side-by-side scrolling
- dark/light themes
- responsive fallback to inline diff when the panel is too narrow

Git should generate the underlying repository change information.

The UI should not attempt to infer repository state from Monaco.

For files selected in the Changes list:

```text
selected file
    ↓
Rust diff request
    ↓
structured diff / old content / new content
    ↓
frontend
    ↓
Monaco DiffEditor
```

Implement a clean abstraction around Monaco so it can later be replaced without affecting Git logic.

Do not load entire multi-gigabyte files into Monaco.

Define reasonable thresholds and graceful fallback UI.

---

# 7. Staging

Support:

- stage file
- unstage file
- stage all
- unstage all

Design the underlying API so these can later be added cleanly:

- stage hunk
- unstage hunk
- stage selected lines

Hunk-level staging does not need to block the initial working version if it introduces significant complexity.

---

# 8. Commits

Commit UI should support:

- summary/title
- optional description
- selected staged changes
- commit button
- validation when nothing is staged
- validation when summary is empty

Commit identity should respect Git configuration.

Do not silently rewrite global Git configuration.

If identity is unavailable, display a clear UI explaining what is missing.

---

# 9. Branches

Implement:

- current branch
- list local branches
- list remote-tracking branches
- create branch
- switch branch
- delete safe local branch
- branch search

Potentially destructive operations must clearly explain what will happen.

Do not implement forced deletion silently.

---

# 10. Remote operations

Implement:

- fetch
- push
- pull
- clone

Show operation state visibly.

Examples:

```text
Fetching origin…
Pushing 3 commits…
Everything is up to date.
Authentication expired.
Merge conflict detected.
Network unavailable.
```

Do not freeze the UI during Git network operations.

Use async Rust tasks and events.

Cancellation support is desirable where practical.

---

# 11. Repository discovery

Support:

### Add local repository

Use a native directory picker.

Validate that the selected directory is a Git repository.

### Clone

Allow:

- repository URL
- selecting from authenticated GitHub repositories

Clone destination should use a native directory picker.

### Recent repositories

Store recently opened local repository paths in normal Branchcraft application settings.

If a repository disappears from disk, handle this gracefully.

---

# 12. Error handling

Do not propagate opaque backend errors directly into toast notifications.

Create structured error categories.

For example:

```rust
enum GitErrorKind {
    NotRepository,
    Authentication,
    Network,
    Conflict,
    DirtyWorkingTree,
    PermissionDenied,
    RepositoryNotFound,
    InvalidReference,
    Unknown,
}
```

Frontend errors should contain:

- concise human-readable summary
- optional detailed explanation
- optional recovery action

Example:

```text
Push failed

GitHub rejected the stored credentials.

[Sign in again]
```

rather than:

```text
libgit2 error -16 class=12
```

Detailed diagnostic information may be written to logs as long as secrets are removed.

---

# 13. Logging

Implement structured application logging.

Requirements:

- backend logs
- log levels
- timestamps
- module names
- optional rotating log files

Never log:

- OAuth tokens
- authorization codes
- HTTP Authorization headers
- passwords
- complete credential callback payloads

Create centralized redaction helpers.

Development logging can be verbose.

Production logging should default to useful operational information without exposing private repository contents unnecessarily.

---

# 14. Branchcraft UI design

Branchcraft should feel modern, focused, and intentional.

Avoid:

- excessive gradients
- giant rounded cards
- mobile-style layouts
- huge spacing
- excessive animations
- dashboard aesthetics

Prefer:

- compact desktop density
- clear hierarchy
- strong typography
- subtle borders
- restrained shadows
- keyboard-friendly controls
- excellent dark mode

Use reusable design tokens.

Use CSS variables for:

- background
- raised background
- border
- text
- secondary text
- selection
- accent
- danger
- success
- diff added
- diff removed

Do not hardcode styling independently in every component.

Use system fonts for normal UI and a quality system monospace stack for source text where Monaco allows it.

The visual identity should use the name **Branchcraft** consistently in:

- application title
- About dialog
- Linux desktop metadata
- package metadata
- documentation
- README
- authentication success pages
- application logs where a product name is needed

Avoid unnecessary "GitHub Desktop clone" wording in user-facing UI.

---

# 15. Native integration

On Fedora/GNOME:

- use native file/directory dialogs where possible
- integrate with system dark/light preference if feasible
- open external URLs in the user's normal browser
- use Secret Service for credentials
- support Wayland properly
- avoid requiring X11
- provide a Branchcraft application icon and `.desktop` integration

Packaging target priorities:

1. RPM
2. Flatpak
3. AppImage if straightforward

Do not require Electron.

Suggested identifiers:

```text
Application name: Branchcraft
Binary: branchcraft
Repository: branchcraft
Desktop entry: branchcraft.desktop
Application ID: io.github.<username>.Branchcraft
```

Keep the application ID configurable until the final GitHub owner/organization name is known.

---

# 16. Testing strategy

Every Rust feature module must be independently testable.

Use dependency injection through traits where external systems are involved.

Provide mocks/fakes for:

- AuthService
- GitService
- GitHub API
- SecretStore

Git tests should create temporary repositories.

Test scenarios should include:

- clean repository
- modified file
- new file
- deleted file
- renamed file
- staged changes
- unstaged changes
- mixed staged/unstaged changes
- detached HEAD
- branch switching
- merge conflict
- repository without remote
- invalid remote
- empty repository

Do not make ordinary unit tests require internet access.

OAuth and GitHub API tests should use mocks.

Frontend tests should focus on behavior rather than implementation details.

---

# 17. Agent-friendly development rules

Branchcraft is expected to be worked on by multiple coding agents.

Optimize the repository for parallel implementation.

Rules:

1. Keep module contracts explicit.
2. Avoid broad "utility" modules.
3. Avoid god objects.
4. Features should own their UI.
5. Shared domain types belong in `app-core`.
6. Tauri commands must be thin adapters.
7. Git logic must never live inside Tauri commands.
8. React components must never contain Git implementation logic.
9. Authentication implementation details must not leak outside `auth`.
10. Secret storage implementation details must not leak outside `storage`.

Write an `ARCHITECTURE.md` explaining module ownership.

For each major feature create a small feature specification in:

```text
docs/features/
```

This should make it possible to assign different agents to:

- OAuth
- Git repository model
- status/staging
- diff backend
- Monaco diff frontend
- commits
- branches
- remotes
- repository picker
- GitHub repository browser
- settings
- packaging

without creating major merge conflicts.

---

# 18. Development sequence

Implement incrementally.

## Milestone 0 — foundation

Deliver:

- `branchcraft` Cargo workspace
- Tauri app named Branchcraft
- React/TypeScript frontend
- formatting/linting
- unit test setup
- CI
- architecture documentation

The app only needs to launch.

## Milestone 1 — local repository

Deliver:

- choose local Git repository
- open repository
- repository metadata
- working tree status
- file list

No GitHub login required yet.

## Milestone 2 — diff

Deliver:

- file selection
- backend diff representation
- Monaco diff viewer
- added/modified/deleted file handling

At this milestone, Branchcraft should already be genuinely useful for inspecting local changes.

## Milestone 3 — stage + commit

Deliver:

- stage/unstage file
- stage all/unstage all
- commit form
- commit creation

## Milestone 4 — branches and history

Deliver:

- history
- branch picker
- create branch
- switch branch
- safe delete

## Milestone 5 — GitHub authentication

Deliver:

- OAuth flow
- loopback callback
- secure keyring storage
- current user
- logout

## Milestone 6 — remotes

Deliver:

- fetch
- pull
- push
- authenticated HTTPS Git operations
- useful progress/error states

## Milestone 7 — cloning and GitHub browser

Deliver:

- list GitHub repositories
- search/filter
- clone
- clone by URL

## Milestone 8 — polish and packaging

Deliver:

- keyboard shortcuts
- settings
- system theme
- Branchcraft application metadata
- RPM packaging
- Flatpak packaging if practical
- production logging
- performance review

Do not attempt every milestone simultaneously.

---

# 19. CI and quality requirements

Set up CI immediately.

Required checks:

Rust:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Frontend:

```text
npm run typecheck
npm run lint
npm test
npm run build
```

Add appropriate Tauri build validation.

Prefer strict TypeScript.

Avoid `any` except where an external library makes it genuinely unavoidable.

No warnings should be accepted casually.

---

# 20. Security requirements

Treat the OAuth token as a secret everywhere.

Rules:

- no token in frontend
- no token in URLs
- no token in process arguments
- no token in config
- no token in logs
- no token in crash messages
- no token in Git config

Rust secret values should use a secret wrapper where practical so accidental `Debug` output is avoided.

Validate all paths coming from the frontend.

Do not let arbitrary frontend input become shell commands.

Avoid shell execution entirely unless unavoidable.

If a Git CLI subprocess is eventually needed, use argument arrays rather than shell-interpolated commands.

---

# 21. Repository and project metadata

Use Branchcraft consistently across the codebase.

Expected repository:

```text
branchcraft
```

Expected package/workspace naming:

```text
branchcraft
branchcraft-app-core
branchcraft-auth
branchcraft-git
branchcraft-github
branchcraft-storage
branchcraft-test-support
```

Where Rust package naming conventions or Cargo workspace structure make shorter internal crate names preferable, that is acceptable, but public-facing project metadata should consistently identify the application as Branchcraft.

Suggested Tauri configuration:

```text
productName: Branchcraft
identifier: io.github.<username>.Branchcraft
```

Suggested binary:

```text
branchcraft
```

Suggested Linux desktop entry:

```text
Name=Branchcraft
Exec=branchcraft
```

README title:

```text
# Branchcraft
```

Suggested short project description:

> Branchcraft is a fast, focused desktop Git client for Linux, built with Rust, Tauri, and Monaco.

Do not add a slogan unless needed.

---

# 22. Initial implementation task

Begin by completing **Milestone 0 and Milestone 1 only**.

Before coding:

1. Inspect current stable versions of:
   - Rust
   - Tauri 2
   - React
   - TypeScript
   - Monaco Editor
   - git2
   - Secret Service/keyring libraries suitable for Fedora
2. Select maintained dependencies.
3. Record the choices and rationale in `docs/architecture.md`.

Then implement:

- `branchcraft` workspace structure
- Tauri application named Branchcraft
- React frontend
- native directory picker
- `app-core` domain types
- `git` service abstraction
- libgit2-based Git implementation
- open existing repository
- display repository name/path
- display active branch
- working-tree status
- changed-file list
- tests using temporary repositories

The initial UI should resemble:

```text
┌──────────────────────────────────────────────────────────┐
│ Branchcraft                                              │
│ Repository name                       Current branch     │
├─────────────────────┬────────────────────────────────────┤
│ Changes             │                                    │
│                     │      Select a changed file         │
│ M  src/main.rs      │                                    │
│ A  new-file.rs      │                                    │
│ D  old-file.rs      │                                    │
│                     │                                    │
├─────────────────────┴────────────────────────────────────┤
│ Open Repository                                          │
└──────────────────────────────────────────────────────────┘
```

Do not implement Monaco diff rendering yet unless Milestones 0 and 1 are complete and tested.

---

# 23. Definition of done for the first implementation

The first Branchcraft implementation is complete when:

- the repository is named `branchcraft`
- the application identifies itself as Branchcraft
- the binary is named `branchcraft`
- the application builds on current Fedora
- the Tauri window launches successfully
- the user can choose an existing repository
- the backend opens it safely
- the application identifies the current branch
- modified/new/deleted files appear correctly
- repository logic has automated tests
- frontend-to-backend boundaries are typed
- there are no secrets involved yet
- `cargo test --workspace` passes
- Rust formatting/clippy passes
- frontend typecheck/lint/tests pass
- architecture documentation exists
- development instructions allow a new contributor to build Branchcraft from a clean Fedora installation

At the end, output:

1. a concise description of the implemented architecture
2. the resulting directory tree
3. commands needed to build/test/run Branchcraft on Fedora
4. tests added
5. known limitations
6. exact scope recommended for the next coding-agent task

Do not merely scaffold empty interfaces.

Milestone 1 must work end-to-end.
