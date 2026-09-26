# Branchcraft

Branchcraft is a fast, focused desktop Git client for Linux, built with Rust,
Tauri, and React.

The current implementation completes Milestones 0 through 5: it opens an
existing local Git repository through a native directory picker, shows its
working-tree changes, renders the selected file in a Monaco diff editor, stages
changes, creates commits, browses commit history, and manages local branches
safely. It also signs in to GitHub in the system browser, validates a PKCE and
state-protected loopback callback, retrieves the current account, and stores
the OAuth session only in the operating system keyring. Git and authentication
logic live in independently tested Rust crates; the React UI talks to them only
through typed Tauri service wrappers.

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
instructions, including GitHub OAuth App configuration.

## Project status

- Implemented: workspace foundation, Tauri shell, React UI, native folder
  picker, repository opening, current branch, working-tree status, structured
  file diffs, responsive Monaco split/inline viewing, file/all staging and
  unstaging, commit creation, bounded commit history, local and remote-tracking
  branch discovery, branch search, creation, safe switching and deletion,
  browser-based GitHub OAuth, PKCE/state validation, an ephemeral loopback
  callback, current-user display, Secret Service/native keyring persistence,
  logout, tests, and CI.
- Next: authenticated fetch, pull, and push (Milestone 6).
- Not yet implemented: remote Git operations, cloning, and production packaging
  polish.

