# Local repository feature

## User outcome

The user chooses an existing directory through a native picker. Branchcraft
opens it only if it is a Git repository and then shows its name, canonical path,
and current branch. Missing folders, permissions, and non-repositories produce
actionable structured errors.

## Ownership

- React UI/hook: `frontend/src/features/repositories`
- Typed IPC: `frontend/src/services/repositoryService.ts`
- Tauri adapter: `src-tauri/src/commands/repositories.rs`
- Behavior: `crates/git`
- Models: `crates/app-core`

Repository discovery, recents, creation, and cloning are intentionally outside
this first slice.

