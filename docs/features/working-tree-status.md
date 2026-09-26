# Working-tree status feature

## User outcome

After opening or refreshing a repository, Branchcraft lists tracked modified,
added, deleted, renamed, type-changed, untracked, and conflicted paths. Index and
worktree changes are represented separately so a file can correctly show a
mixed staged/unstaged state.

## Contract

Each `FileStatus` contains a display path, optional previous path, optional
staged change, optional unstaged change, and conflict flag. The backend is the
source of truth. The frontend only maps these values to concise labels and
colors.

Ignored and unmodified files are excluded. Milestone 3 adds staging controls on
top of this contract without collapsing mixed index/worktree state.

