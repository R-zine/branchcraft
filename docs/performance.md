# Performance review

Milestone 8 reviewed the UI, IPC, Git workers, network clients, and release
artifacts for unbounded or startup-sensitive work.

## Current safeguards

- Filesystem and libgit2 work runs on blocking Rust workers; GitHub HTTP and
  authentication use asynchronous clients with connection/overall timeouts.
- Diff payloads are capped at 2 MiB per side, history at 200 commits, recent
  repositories at 12 entries, and GitHub discovery at 1,000 repositories.
- Monaco and its language registrations remain in a lazy feature chunk, so the
  editor is not downloaded or initialized on the welcome screen.
- Search/filtering is local over the bounded GitHub result and does not trigger
  a network request per keystroke.
- Progress channels update existing React feature state instead of expanding a
  global application store.
- Release builds use thin LTO, one codegen unit, symbol stripping, and aborting
  panics. Production frontend source maps are disabled.
- Settings files are capped at 256 KiB and logs rotate at 1 MiB.

## Build observation

The production Vite build keeps the main application bundle near 293 KiB before
compression (about 87 KiB gzip). Monaco remains the large lazy chunk. This is an
intentional tradeoff for the full diff editor; further language-level splitting
is possible if measured startup or first-diff latency warrants it.

## Follow-up opportunities

Cancellation for long remote operations and clone is still desirable. Add it
only with end-to-end libgit2 callback cancellation and explicit partial-clone
recovery semantics; a UI-only cancel control would be misleading.
