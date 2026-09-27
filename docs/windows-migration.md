# Windows migration audit

This audit was completed before the Windows migration changes. It covers the
runtime source, tests, Tauri configuration, CI, packaging, and contributor
documentation. Generated files (`target`, `frontend/node_modules`) and Git's
sample hooks are not application code and were excluded.

## Findings and migration strategy

| Area | Existing assumption or behavior | Strategy and affected files | Risk and verification |
| --- | --- | --- | --- |
| Desktop bundle targets | `src-tauri/tauri.conf.json` selected only RPM and DEB and described Branchcraft as Linux-only. | Make the default target platform-selected, retain RPM-specific metadata, and add an explicit current-user NSIS configuration. Update `src-tauri/Cargo.toml`, `README.md`, and packaging documentation. | Validate the generated `.exe`, Start Menu shortcut, uninstall entry, product/version/icon metadata, and unsigned SmartScreen behavior on Windows 11. |
| CI coverage | Rust and frontend checks ran only on Ubuntu; the production bundle job ran only in Fedora. | Add a Linux/Windows checks matrix and a native Windows NSIS build/upload job while retaining the Fedora RPM job in `.github/workflows/ci.yml`. | A Windows runner is required; cross-compilation is not accepted as evidence. Installer creation may download the NSIS toolset and WebView2 bootstrapper. |
| Credential storage | `SecretStore` was already an implementation-neutral contract. `keyring` 4.2 selected Secret Service on Linux and Windows Credential Manager on Windows, but comments and docs mentioned only Fedora and the contract had no direct reusable test. | Keep `KeyringSecretStore` as the single native adapter, document both backends, and add offline set/get/overwrite/delete/missing-key contract coverage using an in-memory store. Affected: `crates/storage`, `crates/test-support`, authentication and architecture docs. | Native stores can require an unlocked interactive session, so normal CI must not access a real credential manager. Perform focused manual persistence/logout tests on both desktops. |
| Settings paths | The desktop already obtains `app_config_dir()` from Tauri and passes a `PathBuf` into `JsonSettingsStore`; no XDG or home path is hardcoded. Windows has a localized case-insensitive recent-path comparison and write path. | Preserve the store contract and Tauri path resolver. Expand path tests for spaces, Unicode, mixed case on Windows, and locations outside the home directory. Affected: `src-tauri/src/lib.rs`, `crates/storage/src/lib.rs`. | Windows filesystems can be configured case-sensitive. The comparison intentionally follows the normal case-insensitive Windows user-volume behavior and does not claim filesystem identity equivalence. |
| Logging paths | The logger already receives Tauri's `app_log_dir()` and uses `Path`/`PathBuf`; filenames are Windows-safe. Rotation was untested. | Preserve the existing logger and add isolated creation/rotation tests where global logger installation is not required. Affected: `src-tauri/src/logging.rs`. | Antivirus or another process can temporarily lock a log during replacement on Windows. Failure remains non-fatal and emits no sensitive path or payload. |
| Native dialogs and browser launch | Directory selection uses `tauri-plugin-dialog`; OAuth browser launch uses `tauri-plugin-opener`. No `xdg-open`, `gio`, `start`, Explorer, PowerShell, or shell subprocess is used at runtime. | Keep these common adapters and document Windows browser/dialog smoke tests. Affected: frontend services, `src-tauri/src/lib.rs`, smoke-test docs. | Default-browser policy and endpoint security software are external to Branchcraft and require manual verification. |
| Git filesystem paths | Repository roots already cross IPC as opaque strings and become `PathBuf` in thin commands. Git-relative status paths are normalized to Git's `/` representation before IPC. | Keep native path semantics in Rust. Centralize frontend presentation of Git-relative paths so it cannot be confused with parsing a native repository path. Expand temporary-repository coverage for spaces, Unicode, punctuation, nesting, and mixed case. | Windows reserved names and maximum path policy remain OS constraints. Tests use paths comfortably below legacy limits; a deep-path case verifies application handling where the host allows creation. |
| Frontend path handling | `ChangesPanel` and Monaco language detection split status paths on `/`. These are Git-relative paths normalized by the backend, not native paths, but the distinction was implicit. | Move this behavior to a tested `gitPath` presentation helper. Repository roots remain opaque and are never joined or normalized in TypeScript. Affected: `frontend/src/features/changes`, `frontend/src/features/diff`, new helper tests. | A Git repository created on a case-sensitive platform may contain names that cannot both be checked out on normal Windows volumes; the UI must display backend results without inventing filesystem rules. |
| Line endings and diffs | Diff content and hunks come from libgit2 in Rust; the frontend does not re-diff. No explicit LF/CRLF or `.gitattributes` regression tests existed. | Add shared Git tests for LF, CRLF, mixed working-tree changes, and attribute/config-driven behavior. Preserve raw content for Monaco and libgit2's status/diff decisions; add no custom normalization policy. Affected: `crates/git` tests. | WebView2 and WebKitGTK presentation still need smoke testing. Attribute filters beyond built-in end-of-line conversion may depend on user-installed filter drivers and are outside normal CI. |
| Git implementation | Runtime Git operations and test repository setup already use `git2`; no Git CLI or shell utilities are required. Remote tests use local bare repositories and authentication tests use fakes. | Retain `GitService`/`LibGitService` and run the same suite on Linux and Windows. | Network, proxy, TLS root, and real GitHub permission behavior need manual smoke tests; normal tests intentionally remain offline. |
| OAuth loopback | The common backend already opens the system browser, binds `127.0.0.1:0`, validates an exact callback path and constant-time state, uses PKCE, supports cancellation, and stores only through `SecretStore`. | Keep one implementation for both platforms and run its fake-browser tests in the OS matrix. Document manual Windows firewall/browser tests. Affected: `crates/auth`, `src-tauri`, authentication docs. | Firewall/browser policies and real GitHub configuration cannot be made reliable in pull-request CI. No callback query, code, verifier, token, or authorization header may be logged. |
| Platform-specific assets | `.desktop`, AppStream, RPM, and Flatpak files are intentionally Linux-specific. Windows already has an ICO asset but no installer job. | Leave Linux assets under `packaging/`; use the existing visual identity and ICO for NSIS. Add Windows distribution instructions without adding MSIX, Store, signing, or updater infrastructure. | The placeholder application identifier must be replaced consistently before public releases on either platform. |
| Documentation | Setup, architecture, authentication, and packaging instructions were Fedora-centric. | Split prerequisites and commands by Fedora and Windows in `README.md`, `docs/development.md`, `docs/architecture.md`, `docs/auth.md`, and `docs/packaging.md`; add paired smoke-test checklists. | Toolchain prerequisite names and download UI can evolve; CI commands are the canonical executable specification. |
| Toolchain contract | The workspace declared Rust 1.85, while the pinned `keyring` 4.2 package requires Rust 1.88. | Raise the workspace MSRV to 1.88 and document stable Rust on both platforms. Affected: `Cargo.toml`, `docs/development.md`, `docs/architecture.md`. | CI follows stable; a separate MSRV job is not added because Tauri release builds and the two native stable jobs are the support gate. |

## Audit details

No application runtime use was found for:

- `std::os::unix`, `/tmp`, `$HOME`, `$XDG_*`, or `~/.config`;
- `bash`, `sh`, `grep`, `sed`, `awk`, `xdg-open`, `gio`, PowerShell,
  `explorer.exe`, or `cmd.exe`;
- DBus, Wayland, Secret Service, `.desktop`, RPM, or Flatpak APIs outside the
  native keyring dependency, documentation, CI, and Linux packaging assets;
- Unix permission bits, executable-bit mutation, or symlink creation;
- a Git CLI dependency in production or tests.

`fs::symlink_metadata` and `read_link` in the diff loader are portable Rust APIs.
They allow a symlink to be previewed safely where the host repository contains
one; Branchcraft does not assume that creating symlinks is permitted on
Windows.

The production filesystem boundary is already narrow:

```text
React (opaque repository-root string)
  -> typed Tauri request
  -> PathBuf in a thin command
  -> GitService / SettingsStore using Path and PathBuf
```

Status entries are a separate domain: they are repository-relative Git paths,
serialized with `/` on every platform. Frontend code may present those values
but must not use them to resolve native files.

## Intentionally platform-specific behavior

- Fedora continues to use WebKitGTK, Secret Service/GNOME Keyring, generated
  desktop integration, RPM, and optional Flatpak packaging.
- Windows uses WebView2, Windows Credential Manager through `keyring`, the ICO
  asset, and a current-user NSIS installer with normal uninstall metadata.
- Settings and logs live in the directories returned by Tauri for the running
  platform. Existing Fedora locations are therefore preserved.
- Settings path de-duplication is case-sensitive on Linux and ASCII
  case-insensitive on Windows. This is localized inside the storage adapter.
- Packaging scripts may be platform-specific. Runtime application features may
  not depend on a shell or external Git installation.

## Required automated coverage

Both Linux and Windows CI run Rust formatting, clippy, all workspace tests,
frontend type checking, lint, unit/component tests, and a production frontend
build. Fedora additionally builds an RPM; a native Windows runner builds an
unsigned NSIS installer. Shared tests cover temporary repositories, secret
store semantics, settings, OAuth with fakes, and Git operations without the
network or user credential manager.

## Required manual coverage

Headless CI cannot reliably prove native keyring unlock prompts, real default
browser policy, GitHub OAuth consent, WebView2/WebKitGTK rendering, Start Menu
integration, uninstall behavior, SmartScreen messaging, or real remote
permissions. The paired checklists in `docs/testing/windows-smoke-test.md` and
`docs/testing/linux-smoke-test.md` are release requirements for those areas.
