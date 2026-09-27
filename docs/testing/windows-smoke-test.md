# Windows 11 smoke test

Run this checklist for every release candidate on a supported Windows 11 x64
desktop. Record the Branchcraft version, Windows build, WebView2 runtime
version, tester, date, and repository/remote used. Use a disposable GitHub
repository and never paste credentials into the report.

## Installation and launch

- [ ] Build or download the unsigned NSIS `*-setup.exe` artifact.
- [ ] Run the installer without elevation and acknowledge the expected
  unknown-publisher/SmartScreen warning if Windows shows one.
- [ ] Confirm Branchcraft appears in the Start Menu with the correct name and
  icon, then launch it from that shortcut.
- [ ] Confirm the window title is Branchcraft and WebView2 renders the complete
  welcome screen without blank panels or developer errors.
- [ ] Close and relaunch the application normally.

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
- [ ] Confirm a per-user log is created and no repository content, OAuth data,
  or authorization header appears in it.
- [ ] Uninstall Branchcraft from Windows Settings or the Start Menu entry and
  confirm the executable and shortcut are removed.
- [ ] Confirm uninstall does not silently delete user repositories.
