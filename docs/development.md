# Development

## Fedora prerequisites

Tauri 2 uses WebKitGTK 4.1. Install the official Fedora prerequisites:

```bash
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel gnome-keyring
sudo dnf group install "c-development"
```

Install Rust through rustup and select stable:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install stable --component clippy,rustfmt
```

Install Node.js 22.12 or newer using the Fedora package/module or your normal
version manager. Node 24 LTS is the CI version.

## Install frontend dependencies

```bash
cd frontend
npm ci
cd ..
```

## Quality checks

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd frontend
npm run typecheck
npm run lint
npm test
npm run build
```

Rust repository tests create isolated temporary repositories and never use the
network or modify global Git configuration.

## Run

For GitHub sign-in, create a GitHub OAuth App with the callback URL
`http://127.0.0.1/oauth/callback`, then export its application configuration:

```bash
export BRANCHCRAFT_GITHUB_CLIENT_ID=your_client_id
export BRANCHCRAFT_GITHUB_CLIENT_SECRET=your_client_secret
```

GitHub currently requires the client secret during OAuth App token exchange,
even for native apps using PKCE. It is public application configuration in a
desktop binary, not a confidential secret. See [auth.md](auth.md) for the flow,
security properties, and packaging guidance.

```bash
cargo install tauri-cli --version '^2.11' --locked
cargo tauri dev --config src-tauri/tauri.conf.json
```

The Tauri command starts Vite automatically. To build an RPM:

```bash
cargo tauri build --config src-tauri/tauri.conf.json --bundles rpm
```

The package is written below `target/release/bundle/rpm`. The application ID is
currently the explicit placeholder `io.github.username.Branchcraft`; replace the
`username` segment in `src-tauri/tauri.conf.json` when the final GitHub owner is
known.

The desktop session must provide a Secret Service implementation. Fedora GNOME
normally starts GNOME Keyring automatically. If the keyring is locked or
unavailable, local Git features continue to work and the account menu presents
a recoverable credential-store error.

## Wayland

No X11-specific code or Electron runtime is used. On Fedora/GNOME the Tauri
WebKitGTK window and native dialog use the desktop's normal Wayland-capable
stack. `libxdo-devel` is installed because it remains in Tauri's general Fedora
prerequisite list, not because Branchcraft calls it directly.

