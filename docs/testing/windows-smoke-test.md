# Windows 11 smoke test

Run this checklist for every release candidate on a supported Windows 11 x64
desktop. Record the Branchcraft version, Windows build, WebView2 runtime
version, tester, date, and repository/remote used. Use a disposable GitHub
repository and never paste credentials into the report.

## Milestone 8 verification (2026-10-25)

**Status: complete on Windows 11.** This run verified the Windows-specific
Milestone 8 scope using the release installer produced from the current source
tree. Fedora/Linux verification remains separate.

| Field | Value |
| --- | --- |
| Branchcraft | 0.1.0 |
| Guest | Windows 11 Pro x64, build 22000 |
| Hypervisor | QEMU 11.0.50 with WHPX |
| WebView2 Runtime | 155.0.4283.62 |
| Artifact | `Branchcraft_0.1.0_x64-setup.exe` |
| SHA-256 | `f176d5fadf438c570392f0113c11ca67fcd3dbe410f8cb3d8c58a1287d3d3214` |
| Signature | Intentionally unsigned |
| Test environment | Disposable QEMU release-test overlay |

Verified in the installed application:

- the NSIS release build completed and installed per-user under
  `%LOCALAPPDATA%\Branchcraft`;
- executable version, uninstall metadata, application name, icon, and Start
  Menu shortcut were correct;
- launch from the installed shortcut produced a responsive Branchcraft window
  and a complete WebView2-rendered welcome screen without blank panels or a
  developer-error overlay;
- `Ctrl+,` opened Settings in the installed build;
- the system appearance rendered correctly, and dark mode with the Monochrome
  palette rendered with readable UI colors;
- the selected theme and palette were written to the bounded settings file and
  remained active after a normal close and Start Menu relaunch;
- the per-user production log contained only the expected runtime-start record;
- silent uninstall returned success and removed the executable, Start Menu
  shortcut, and uninstall registration;
- uninstall preserved both application settings and a user-repository marker
  outside the installation directory; and
- the repository's formatting, lint, Rust, frontend, and production-build gates
  passed before packaging.

The broader checklist below remains the release-candidate checklist for changes
to repository workflows, GitHub OAuth, live remotes, native dialogs, or Windows
policy behavior. Those areas have automated coverage but were not re-exercised
against a live GitHub account during this Milestone 8 packaging run.

## Installation and launch

- [x] Build or download the unsigned NSIS `*-setup.exe` artifact.
- [ ] Run the installer without elevation and acknowledge the expected
  unknown-publisher/SmartScreen warning if Windows shows one.
- [x] Confirm Branchcraft appears in the Start Menu with the correct name and
  icon, then launch it from that shortcut.
- [x] Confirm the window title is Branchcraft and WebView2 renders the complete
  welcome screen without blank panels or developer errors.
- [x] Close and relaunch the application normally.

## Local repository workflow

- [ ] Open a clean local repository with the native directory picker.
- [ ] Open a repository whose path contains spaces, for example
  `C:\Users\Test User\Projects\branchcraft test`.
- [ ] If another drive is available, open a repository such as
  `D:\Code\example` to confirm there is no home-directory assumption.
- [ ] Confirm current branch, clean/changed status, nested paths, Unicode
  filenames, and mixed-case paths are displayed correctly.
- [ ] Modify LF and CRLF files, including a repository with `.gitattributes`,
  and confirm unchanged lines are not all reported as changed.
- [ ] Inspect modified, new, deleted, renamed, binary, and large-file diffs.
- [ ] Verify Monaco syntax highlighting, line numbers, selection/copy,
  keyboard navigation, resize, scrolling, synchronized side-by-side scrolling,
  inline mode, and side-by-side mode.
- [ ] Stage and unstage one file, then stage and unstage all files.
- [ ] Create a commit and confirm it appears in history.
- [ ] Create and switch to a branch; switch back and safely delete the merged
  branch.
- [ ] Confirm a detached HEAD and a repository without a remote show safe,
  understandable states.
- [ ] Confirm unresolved conflicts are shown and unsafe pull/checkout actions
  are refused.

## GitHub and remotes

- [ ] Start GitHub sign-in and confirm the default browser opens.
- [ ] Complete authorization through the random `127.0.0.1` callback and
  confirm the browser returns a safe success page.
- [ ] Cancel a second sign-in attempt and confirm Branchcraft remains usable.
- [ ] Restart Branchcraft and confirm the account persists through Windows
  Credential Manager without another browser authorization.
- [ ] Browse and search accessible GitHub repositories.
- [ ] Clone a repository through the native destination picker.
- [ ] Fetch, pull, and push against the disposable remote.
- [ ] Confirm authentication/network errors do not reveal an authorization
  code, token, callback query, verifier, header, or credential-bearing URL.
- [ ] Log out, restart, and confirm the account is gone from Branchcraft.
- [ ] Authenticate again successfully.

## UI, persistence, and uninstall

- [ ] Switch among system, light, and dark themes; confirm Monaco and the
  surrounding UI use matching readable colors.
- [ ] Restart and confirm theme and recent repositories persist.
- [x] Confirm a per-user log is created and no repository content, OAuth data,
  or authorization header appears in it.
- [ ] Uninstall Branchcraft from Windows Settings or the Start Menu entry and
  confirm the executable and shortcut are removed.
- [x] Confirm uninstall does not silently delete user repositories.
