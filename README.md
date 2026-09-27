# Branchcraft

Branchcraft is a fast, focused desktop Git client for Fedora Linux and Windows,
built with Rust, Tauri, and React.

The current implementation completes Milestones 0 through 8: it opens an
existing local Git repository through a native directory picker, shows its
working-tree changes, renders the selected file in a Monaco diff editor, stages
changes, creates commits, browses commit history, and manages local branches
safely. It fetches, pulls, and pushes local or authenticated GitHub HTTPS
remotes without freezing the UI, with streamed progress and recoverable error
states. It also signs in to GitHub in the system browser, validates a PKCE and
state-protected loopback callback, retrieves the current account, and stores
the OAuth session only in the operating system keyring. Signed-in users can
browse, search, and filter their accessible GitHub repositories, then clone one
through a native destination picker. A repository can also be cloned directly
from a validated HTTPS URL. Branchcraft also persists non-secret theme and
recent-repository settings, follows the desktop color scheme by default,
provides keyboard navigation, and includes RPM, NSIS, and practical Flatpak
packaging assets. Git, authentication, and settings logic live in independently
tested Rust crates; the React UI talks to them only through typed Tauri service
wrappers.

## Quick start on Fedora

Install the native development packages:

```bash
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel
sudo dnf group install "c-development"
```

Install stable Rust (1.88 or newer) and Node.js 22.12 or newer, then:

```bash
cd frontend
npm ci
npm run typecheck
npm run lint
npm test
npm run build
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

## Quick start on Windows 11

Install the Desktop development with C++ workload from Visual Studio 2022 Build
Tools, the current stable Rust MSVC toolchain, Node.js 22.12 or newer, and the
WebView2 Runtime. In a Developer PowerShell:

```powershell
cd frontend
npm ci
npm run typecheck
npm run lint
npm test
npm run build
cd ..

cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo run -p branchcraft
```

For hot reload or an unsigned per-user installer:

```powershell
cargo install tauri-cli --version '^2.11' --locked
cargo tauri dev --config src-tauri/tauri.conf.json
cargo tauri build --config src-tauri/tauri.conf.json --bundles nsis
```

The installer is written below `target\release\bundle\nsis`. Windows may show
an expected unknown-publisher or SmartScreen warning because release builds are
not code-signed.

See [docs/development.md](docs/development.md) for complete setup and packaging
instructions, including GitHub OAuth App configuration.

To enable the repository's full test, lint, formatting, type-check, and build
suite before every commit, run this once after cloning:

```text
git config core.hooksPath .githooks
```

## Project status

- Implemented: workspace foundation, Tauri shell, React UI, native folder
  picker, repository opening, current branch, working-tree status, structured
  file diffs, responsive Monaco split/inline viewing, file/all staging and
  unstaging, commit creation, bounded commit history, local and remote-tracking
  branch discovery, branch search, creation, safe switching and deletion,
  browser-based GitHub OAuth, PKCE/state validation, an ephemeral loopback
  callback, current-user display, native credential persistence through Secret
  Service or Windows Credential Manager, logout, authenticated HTTPS
  fetch/pull/push, progress reporting, safe
  fast-forward/merge handling, authenticated GitHub repository browsing,
  repository search/filtering, clone-by-selection and clone-by-URL, native
  destination selection, clone progress, recent repositories, keyboard
  shortcuts, system/light/dark themes, native window-theme synchronization,
  settings recovery, production logging, release optimization, application
  metadata, RPM and unsigned NSIS packaging, Flatpak packaging assets,
  cross-platform tests, and Linux/Windows CI.
- Release preparation remaining outside the implementation: select the final
  GitHub owner/application ID and configure release OAuth credentials. Code
  signing is optional and is not required for the unsigned release workflow.

