# Staging and commits (Milestone 3)

## User outcome

Each changed file has a staging checkbox. Fully staged files are checked,
unstaged files are unchecked, and files with both index and worktree changes
show a mixed state. Users can stage or unstage one file, stage all worktree
changes, or unstage the complete index. These operations never discard working
files.

The commit form accepts a required summary and optional description and reports
the number of staged files. A successful commit clears the form, displays the
new short commit ID, refreshes repository status, and keeps any unstaged edits
visible.

## Backend contract

`GitService` owns `stage_file`, `unstage_file`, `stage_all`, `unstage_all`, and
`commit`. Paths are validated as normal repository-relative paths before they
reach libgit2. Staging handles additions, modifications, deletions, conflicts,
and both sides of renames. Unstaging restores index entries from `HEAD`; on an
unborn branch it removes them from the index while leaving the working tree
untouched.

Commit creation writes exactly the current index. The summary is trimmed and
the optional description is separated by a blank line. The effective Git
configuration supplies author and committer identity. Branchcraft does not
write `user.name` or `user.email` automatically.

Structured errors distinguish empty summaries, no staged changes, missing or
invalid identity, unresolved conflicts, permissions, and unexpected Git
failures.

## Frontend ownership

- `features/changes/useStaging.ts` owns staging operation state and errors.
- `features/commit` owns form state, local validation, and success feedback.
- `services/stagingService.ts` and `services/commitService.ts` are the raw Tauri
  invocation boundaries.
- Mutation commands return refreshed repository snapshots, so file selection,
  staging counts, and diffs stay synchronized.

Hunk-level and selected-line staging remain intentionally deferred.
