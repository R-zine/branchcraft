# Development

Branchcraft supports Fedora Linux and Windows 11 from the same source tree.
Commands below run from the repository root unless a section says otherwise.
The workspace follows stable Rust and has an MSRV of 1.88; CI uses Node 24,
while `frontend/package.json` accepts Node 22.12 or newer.

## Fedora prerequisites

Tauri 2 uses WebKitGTK 4.1. Install the Fedora development packages and a
Secret Service implementation:

```bash
sudo dnf check-update
sudo dnf install webkit2gtk4.1-devel openssl-devel curl wget file \
  libappindicator-gtk3-devel librsvg2-devel libxdo-devel gnome-keyring \
  git nodejs npm
sudo dnf group install "c-development"
```

Install Rust through rustup and select stable:

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install stable --component clippy,rustfmt
```

Fedora GNOME normally starts GNOME Keyring automatically. A locked or missing
keyring affects GitHub sign-in only; local Git features remain available and
Branchcraft reports a recoverable credential-store error.

## Windows 11 prerequisites

Install these x64 tools:

1. Visual Studio 2022 Build Tools with the **Desktop development with C++**
   workload, MSVC build tools, and a Windows 10 or Windows 11 SDK.
2. Rustup with the stable `x86_64-pc-windows-msvc` toolchain and the `clippy`
   and `rustfmt` components.
3. Node.js 22.12 or newer (Node 24 is used in CI).
4. Microsoft Edge WebView2 Runtime. It is normally present on Windows 11; the
   NSIS installer uses Tauri's bootstrapper behavior if it is missing.
5. Git for cloning the Branchcraft source tree. Branchcraft itself does not
   call Git for Windows at runtime; all product Git operations use libgit2.

After installing Rust, a Developer PowerShell can verify the target:

```powershell
rustup default stable-x86_64-pc-windows-msvc
rustup component add clippy rustfmt
rustc --version
node --version
npm --version
```

No Bash, WSL, Unix command-line tools, or external Git executable is required
to build or run application features.

## Install frontend dependencies

Fedora:

```bash
cd frontend
npm ci
cd ..
```

Windows PowerShell:

```powershell
Set-Location frontend
npm ci
Set-Location ..
```

## Quality checks

Run the same checks on both platforms:

```text
cargo fmt --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace

cd frontend
npm run typecheck
npm run lint
npm test
npm run build
```

Use the shell's normal `cd`/`Set-Location` command for the displayed sequence.
Rust repository tests create isolated repositories with libgit2 and do not use
the network, a global Git identity, a GitHub account, or the native credential
manager. The same test suite runs on Linux and Windows in CI.

### Pre-commit hook

Branchcraft includes a repository-managed pre-commit hook that runs the full
platform-neutral CI check suite plus a staged whitespace check. Enable it once
after each clone from the repository root:

```text
git config core.hooksPath .githooks
```

The hook runs every check even when an earlier command fails, then reports all
failed gates together. It expects the Rust toolchain, frontend dependencies,
and Node.js described above to be installed. Git preserves the executable bit
on Linux; if a non-Git file transfer removed it, restore it with
`chmod +x .githooks/pre-commit`.

## Run Branchcraft

Install the pinned Tauri CLI line once:

```text
cargo install tauri-cli --version '^2.11' --locked
```

Then start Vite and the desktop shell together on either platform:

```text
cargo tauri dev --config src-tauri/tauri.conf.json
```

Native directory selection is provided by the Tauri dialog plugin. The OAuth
authorization page is opened by the Tauri opener plugin. Runtime code does not
invoke `xdg-open`, `gio`, `start`, Explorer, PowerShell, or a shell.

## GitHub OAuth development configuration

Create a GitHub OAuth App with this callback URL:

```text
http://127.0.0.1/oauth/callback
```

Branchcraft selects an ephemeral port for each attempt. On Fedora, set the
development values in the launching shell:

```bash
export BRANCHCRAFT_GITHUB_CLIENT_ID=your_client_id
export BRANCHCRAFT_GITHUB_CLIENT_SECRET=your_client_secret
```

On Windows PowerShell:

```powershell
$env:BRANCHCRAFT_GITHUB_CLIENT_ID = "your_client_id"
$env:BRANCHCRAFT_GITHUB_CLIENT_SECRET = "your_client_secret"
```

GitHub currently requires the client secret during OAuth App token exchange,
even for native apps using PKCE. It is public application configuration in a
desktop binary, not a confidential secret. A build without these two values
still supports all local Git workflows and reports GitHub sign-in as
unconfigured. See [auth.md](auth.md) for the security model.

## Build packages

Fedora's primary artifact is an RPM:

```bash
cargo tauri build --config src-tauri/tauri.conf.json --bundles rpm
```

The result is below `target/release/bundle/rpm`.

Windows' primary artifact is an unsigned, per-user NSIS installer:

```powershell
cargo tauri build --config src-tauri/tauri.conf.json --bundles nsis
```

The result is below `target\release\bundle\nsis` and normally has a name such
as `Branchcraft_0.1.0_x64-setup.exe`. It creates a Start Menu shortcut and a
normal uninstall entry. No certificate is required; Windows may show an
unknown-publisher or SmartScreen warning for an unsigned build.

The unqualified `cargo tauri build --config src-tauri/tauri.conf.json` asks
Tauri for all bundle types supported by the host. Release and CI instructions
use `--bundles` explicitly to avoid unnecessary artifacts.

The application ID remains the explicit placeholder
`io.github.username.Branchcraft`. Before a public release, replace `username`
consistently in the Tauri config, AppStream metadata, Flatpak assets, and
workspace repository URL. See [packaging.md](packaging.md).

## Platform data and logs

The desktop adapter asks Tauri for the current platform's application config
and log directories. It never constructs an XDG path, `%APPDATA%` path, or home
directory manually. The JSON settings file contains only theme and recent
repository metadata. OAuth sessions are physically separate:

- Fedora: Secret Service/GNOME Keyring through `keyring`.
- Windows: Windows Credential Manager through `keyring`.

Logs rotate at 1 MiB and never intentionally include repository paths,
credentials, server responses, callback queries, or diff contents.

## Display engines

Fedora uses WebKitGTK in the desktop's normal Wayland-capable stack. Windows
uses WebView2. The React and Monaco implementation is shared; engine-specific
workarounds must be narrowly scoped and documented. Use the paired release
checklists:

- [Fedora smoke test](testing/linux-smoke-test.md)
- [Windows smoke test](testing/windows-smoke-test.md)
