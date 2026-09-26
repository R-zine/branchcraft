# Diff feature (Milestone 2)

Milestone 2 provides structured backend diffs and a replaceable Monaco adapter.
Selecting a changed file requests a comparison between its `HEAD` version and
its current working-tree version. This shows the complete local change,
including staged and unstaged edits, while the Changes list continues to
preserve those states independently.

## Backend contract

`branchcraft-app-core` owns the serializable `Diff`, `DiffFile`, `DiffHunk`, and
`DiffLine` models. `branchcraft-git` validates the repository-relative file
path, reads the old side from the `HEAD` tree, reads the new side from the work
tree (or index when appropriate), and asks libgit2 to generate structured
hunks. The Tauri command is only an asynchronous adapter and runs blocking
filesystem/libgit2 work off the webview thread.

Added, modified, deleted, renamed, type-changed, and conflicted status kinds
are represented. Empty files are valid text previews. Rename metadata preserves
the previous path so the correct `HEAD` blob is compared with the new path.

## Preview safety

Each side is limited to 2 MiB. The backend checks filesystem metadata or the
Git object header before loading content, and does not send source bytes to the
frontend when either side exceeds the limit. Content containing NUL bytes or
invalid UTF-8 is classified as binary. Non-blob/non-regular content is marked
unsupported. All three cases receive a lightweight explanation and byte sizes
instead of opening Monaco.

Paths supplied over IPC must be normal, relative repository paths. Traversal,
absolute paths, and working-tree resolutions outside the selected repository
are rejected.

## Frontend behavior

The diff feature owns its typed service, loading hook, path-to-language mapping,
fallback states, and Monaco wrapper. Monaco is loaded only after a previewable
file is selected. The editor provides syntax highlighting, line numbers,
word-level change rendering, horizontal scrolling, synchronized split panes,
light/dark themes, and explicit Split/Inline controls. Panels narrower than
720 pixels automatically use inline mode.

