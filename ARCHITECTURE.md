# Branchcraft architecture

The detailed architecture, dependency decisions, module boundaries, and data
flow are maintained in [docs/architecture.md](docs/architecture.md).

Ownership is intentionally split by directory:

| Area | Owner | Contract |
| --- | --- | --- |
| Domain models | `crates/app-core` | Serializable, implementation-neutral types |
| Git behavior | `crates/git` | `GitService`; no Tauri or React dependencies |
| Desktop adapter | `src-tauri` | Thin commands and native plugins only |
| Frontend features | `frontend/src/features` | Feature-local UI and state |
| Frontend IPC | `frontend/src/services` | The only location for raw Tauri invocation |
| Test fixtures | `crates/test-support` | Reusable, offline temporary repositories |

Future authentication, GitHub API, and preference storage crates must preserve
these boundaries. Credentials never cross the Tauri command boundary.

