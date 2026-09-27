# Branchcraft architecture

The detailed architecture, dependency decisions, module boundaries, and data
flow are maintained in [docs/architecture.md](docs/architecture.md).

Ownership is intentionally split by directory:

| Area | Owner | Contract |
| --- | --- | --- |
| Domain models | `crates/app-core` | Serializable, implementation-neutral types |
| Authentication | `crates/auth` | OAuth/PKCE lifecycle and backend-only session access |
| Persistence | `crates/storage` | Secret Service/Windows Credential Manager plus non-secret JSON settings |
| GitHub HTTP API | `crates/github` | GitHub-specific HTTP operations; no local Git behavior |
| Git behavior | `crates/git` | `GitService`; no Tauri or React dependencies |
| Desktop adapter | `src-tauri` | Thin commands and native plugins only |
| Frontend features | `frontend/src/features` | Feature-local UI and state |
| Frontend IPC | `frontend/src/services` | The only location for raw Tauri invocation |
| Test fixtures | `crates/test-support` | Reusable, offline temporary repositories |

Credentials never cross the Tauri command boundary. Remote Git behavior
requests an ephemeral `SecretString` through `AuthService` and supplies it to
libgit2's credential callback only for exact, credential-free
`https://github.com` remotes. Tokens are never placed in a remote URL.
The same boundary applies to cloning: the GitHub repository browser receives
only public repository metadata, while an optional access token stays in Rust
and is exposed only to the exact GitHub HTTPS credential callback.
Ordinary preferences are stored separately from credentials. Theme and recent
repository metadata use the application configuration directory; OAuth tokens
remain exclusively in the operating-system keyring.
Tauri resolves config/log directories and native dialogs on Fedora and Windows;
the domain crates contain no XDG, home-directory, drive-letter, or shell-command
assumptions.

