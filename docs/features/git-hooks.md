# Git hooks and terminal output

## User outcome

When a repository has a commit hook, Branchcraft opens a terminal-style modal
as soon as the hook starts. Standard output and standard error stream into the
window while the commit remains busy. The completed session stays available
until the user closes it, making both successful quality checks and failures
inspectable. Repositories without hooks commit normally without opening the
terminal.

## Commit lifecycle

Branchcraft runs the standard client-side commit hooks around its libgit2
commit implementation:

| Hook | Arguments | Failure behavior |
| --- | --- | --- |
| `pre-commit` | none | Stops the commit. |
| `prepare-commit-msg` | message file, `message` | Stops the commit; message edits are preserved. |
| `commit-msg` | message file | Stops the commit; message edits are preserved. |
| `post-commit` | none | Reported in the terminal but cannot undo the completed commit. |

Hooks are discovered in `.git/hooks` unless `core.hooksPath` selects another
directory. Relative configured paths are resolved from the repository
worktree. Before writing the commit, Branchcraft reopens and validates the
index so intentional changes made by a hook are included safely.

## Execution and safety

Hooks are programs supplied by the repository and therefore can execute
arbitrary code with the user's permissions, just as they do in command-line
Git. Branchcraft does not silently download or install hooks. On Linux, a hook
must have an executable bit. On Windows, extensionless hooks use their shebang;
common Git for Windows `sh`/`bash` installations are resolved even when the
interpreter is not on the desktop process's `PATH`.

Each hook runs with the repository as its working directory and receives
`GIT_DIR`, `GIT_WORK_TREE`, `GIT_EDITOR`, and `BRANCHCRAFT` environment values.
Process output is decoded lossily for display, never interpreted as HTML, and
is capped at 1 MiB per hook while the underlying pipes continue to drain.

## Ownership

- `crates/hooks` owns discovery, execution, lifecycle events, output bounds,
  and platform launch behavior.
- `crates/git` composes hooks with commit validation and libgit2 commit writes.
- `src-tauri/src/commands/commits.rs` forwards typed hook events through an IPC
  channel from the blocking worker.
- `frontend/src/features/commit` owns terminal session state and presentation.
