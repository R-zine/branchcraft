# Branches and history (Milestone 4)

## User outcome

The Changes/History tabs let users inspect the newest commits reachable from
the current `HEAD`. Each entry shows its summary, optional description, author,
local time, and short commit ID. Empty and detached repositories are handled
without special setup, and history is capped at the newest 200 commits to keep
the desktop UI responsive.

The header branch picker lists and searches local and remote-tracking branches.
Users can create a local branch at the current commit or switch to another
local branch. Creation intentionally leaves the current branch unchanged.
Remote-tracking branches are visible for context and remain read-only in the
branch picker; fetch, pull, and push update them through the remote workflow.

Deleting a local branch requires an explicit confirmation. The backend rejects
the current branch and branches with commits that are not merged into the
current `HEAD`; Branchcraft has no forced-delete action.

## Backend contract

`GitService` owns `history`, `branches`, `create_branch`, `checkout_branch`, and
`delete_branch`. The implementation uses libgit2 directly and never executes a
shell. Branch names are validated by libgit2, only local branches can be
mutated, and checkout uses the safe strategy so conflicting working-tree edits
are preserved and reported as a structured `DirtyWorkingTree` error.

History returns implementation-neutral commit metadata and a `truncated` flag.
Branch mutations return a `BranchOperationResult` containing both the refreshed
repository snapshot and the new branch list, preventing stale header or working
tree state after a switch.

## Frontend ownership

- `features/history` owns history loading, retry, empty, and list states.
- `features/branches` owns the branch popover, filtering, mutation state, and
  safe-delete confirmation.
- `services/historyService.ts` and `services/branchService.ts` are the only raw
  Tauri invocation boundaries for these features.
- Repository revision changes invalidate history and diffs after mutations.

Remote branch checkout, merge/rebase, branch comparison, pagination, and
history commit-detail diffs remain intentionally deferred.
