# Branchcraft

Branchcraft is a fast, focused desktop Git client for Linux, built with Rust,
Tauri, and React.

The current implementation completes Milestones 0 and 1: it opens an existing
local Git repository through a native directory picker and shows the repository
name, path, active branch, and changed files. Git operations live in an
independently tested Rust crate; the React UI talks to it only through typed
Tauri service wrappers.

## Quick start on Fedora

Install the native development packages:

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```

Install the current stable Rust toolchain and Node.js 22.12 or newer, then:

```bash
cd frontend
npm ci
npm run typecheck
npm run lint
npm test
cd ..

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p branchcraft
```

For the development server with hot reload, install the Tauri CLI and run:

```bash
cargo install tauri-cli --version '^2.11' --locked
cargo tauri dev --config src-tauri/tauri.conf.json
```

See [docs/development.md](docs/development.md) for complete setup and packaging
instructions.

## Project status

- Implemented: workspace foundation, Tauri shell, React UI, native folder
  picker, repository opening, current branch, working-tree status, tests, CI.
- Next: structured file diffs and the Monaco diff viewer (Milestone 2).
- Not yet implemented: staging, commits, history, branch management, GitHub
  OAuth, remote operations, cloning, and production packaging polish.

