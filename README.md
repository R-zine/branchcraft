# Branchcraft

Branchcraft is a desktop Git client for Fedora Linux and Windows 11. It is
built with Rust, Tauri, React, and libgit2, with an emphasis on fast local
workflows and clear, recoverable errors.

The application can open or clone repositories, inspect structured diffs,
stage changes, create commits, browse history, manage branches, and fetch,
pull, or push remotes. Commit hooks run in the normal Git order and stream
their output to an in-app terminal. GitHub sign-in is optional; when enabled,
it adds authenticated remotes and repository discovery while keeping tokens in
the operating system credential store.

Branchcraft is currently at version 0.1.0. The local Git workflow is usable,
while release identity, signing, and distribution details are still being
finalized.

## Technology

- **Desktop:** Tauri 2, using WebKitGTK on Fedora and WebView2 on Windows.
- **Backend:** Rust 2024, libgit2, Tokio, and independently tested workspace
  crates.
- **Frontend:** React, TypeScript, Vite, and Monaco Editor.
- **Credentials:** Secret Service/GNOME Keyring on Fedora and Windows
  Credential Manager on Windows.
- **Packaging:** RPM, NSIS, and local Flatpak assets.

## Repository layout

| Path | Responsibility |
| --- | --- |
| `crates/app-core` | Shared domain models and serializable error types. |
| `crates/git` | Repository, diff, staging, history, branch, remote, and clone operations. |
| `crates/hooks` | Git hook discovery, execution, and bounded output streaming. |
| `crates/auth` | GitHub OAuth flow and session orchestration. |
| `crates/github` | GitHub REST API adapter. |
| `crates/storage` | Settings and native credential-store adapters. |
| `crates/test-support` | Temporary repositories and reusable test doubles. |
| `src-tauri` | Desktop entry point, dependency assembly, and Tauri commands. |
| `frontend` | React features, typed IPC services, and application styles. |
| `packaging` | Linux desktop metadata and Flatpak assets. |
| `docs` | Architecture, development, security, feature, and release notes. |

## Start developing

You will need:

- stable Rust with `rustfmt` and `clippy` (minimum supported Rust version:
  1.88);
- Node.js 22.12 or newer and npm;
- the native dependencies required by Tauri on your platform.

Fedora needs WebKitGTK 4.1 and related development packages. Windows needs the
Visual Studio C++ build tools, a Windows SDK, and WebView2. See
[the development guide](docs/development.md) for the exact platform setup.

From the repository root, install the frontend dependencies and Tauri CLI:

```text
npm --prefix frontend ci
cargo install tauri-cli --version '^2.11' --locked
```

Start Vite and the desktop application together:

```text
cargo tauri dev --config src-tauri/tauri.conf.json
```

The app works without GitHub credentials for local repositories. To develop
GitHub sign-in and authenticated remote features, configure the OAuth App and
environment variables described in [docs/auth.md](docs/auth.md).

## How the application is wired

Frontend features call typed service wrappers, which are the only code that
invokes Tauri commands. The commands delegate to Rust services and move
blocking filesystem and libgit2 work off the async runtime:

```text
React feature -> frontend service -> Tauri command -> Rust service -> libgit2 / OS API
```

Domain logic lives in the workspace crates rather than in command handlers or
React components. Credentials never cross into the webview, repository paths
are validated in Rust, and progress for clones, remotes, and Git hooks is sent
to the UI over typed channels.

For the complete data flows and ownership boundaries, read
[docs/architecture.md](docs/architecture.md).

## Run the checks

The backend test suite uses isolated temporary repositories and does not need
network access, a global Git identity, or a GitHub account.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

npm --prefix frontend run typecheck
npm --prefix frontend run lint
npm --prefix frontend test
npm --prefix frontend run build
```

To run the same checks automatically before every commit, enable the included
hook once per clone:

```text
git config core.hooksPath .githooks
```

Linux and Windows CI run the same platform-neutral test suite.

## Build a package

Build an RPM on Fedora:

```text
cargo tauri build --config src-tauri/tauri.conf.json --bundles rpm
```

Build an unsigned, per-user NSIS installer on Windows:

```text
cargo tauri build --config src-tauri/tauri.conf.json --bundles nsis
```

Before publishing packages, replace the placeholder GitHub owner and
application ID throughout the release metadata. See
[docs/packaging.md](docs/packaging.md) for the required files and artifact
locations.

## Documentation

- [Development setup](docs/development.md)
- [Architecture](docs/architecture.md)
- [GitHub authentication and credential handling](docs/auth.md)
- [Packaging](docs/packaging.md)
- [Performance boundaries](docs/performance.md)
- [Feature notes](docs/features/)
- [Fedora smoke test](docs/testing/linux-smoke-test.md)
- [Windows smoke test](docs/testing/windows-smoke-test.md)

## Contributing

Keep platform-independent behavior in the workspace crates and keep Tauri
commands thin. Add tests at the lowest layer that owns a behavior, then run the
full check suite above before opening a pull request. Changes that affect a
native integration or installer should also follow the relevant platform smoke
test.

## License

Branchcraft is available under the [MIT License](LICENSE).
