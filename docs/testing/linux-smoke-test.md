# Fedora Linux smoke test

Run this checklist for every release candidate in a supported Fedora GNOME
desktop session. Record the Branchcraft version, Fedora release, display
session, WebKitGTK version, tester, date, and disposable repository/remote.
Never paste credentials into the report.

## Installation and launch

- [ ] Build the x86_64 RPM and install it with the normal Fedora package tools.
- [ ] Confirm Branchcraft appears in the desktop application launcher with the
  correct name and icon, then launch it from that entry.
- [ ] Confirm the WebKitGTK window renders the complete welcome screen on the
  normal Wayland session without blank panels or developer errors.
- [ ] Close and relaunch the application normally.

## Local repository workflow

- [ ] Open a clean local repository with the native directory picker.
- [ ] Open a repository whose path contains spaces, for example
  `/home/test/Projects/branchcraft test`.
- [ ] Open a repository outside the home directory, such as
  `/tmp/branchcraft-test`, when permissions allow it.
- [ ] Confirm current branch, clean/changed status, nested paths, Unicode
  filenames, punctuation, and mixed-case paths are displayed correctly.
- [ ] Modify LF and CRLF files, including a repository with `.gitattributes`,
  and confirm unchanged lines are not all reported as changed.
- [ ] Inspect modified, new, deleted, renamed, binary, symlink, and large-file
  diffs.
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
- [ ] Restart Branchcraft and confirm the account persists through Secret
  Service/GNOME Keyring without another browser authorization.
- [ ] Browse and search accessible GitHub repositories.
- [ ] Clone a repository through the native destination picker.
- [ ] Fetch, pull, and push against the disposable remote.
- [ ] Confirm authentication/network errors do not reveal an authorization
  code, token, callback query, verifier, header, or credential-bearing URL.
- [ ] Log out, restart, and confirm the account is gone from Branchcraft.
- [ ] Authenticate again successfully.

## UI and persistence

- [ ] Switch among system, light, and dark themes; confirm Monaco and the
  surrounding UI use matching readable colors.
- [ ] Restart and confirm theme and recent repositories persist in the same
  Fedora application-config location used by prior releases.
- [ ] Confirm a per-user log is created and no repository content, OAuth data,
  or authorization header appears in it.
- [ ] Remove the RPM and confirm package-managed application files and the
  launcher entry are removed without deleting user repositories.
- [ ] If Flatpak packaging is being released, repeat launch, picker, keyring,
  OAuth, and local/remote Git checks inside the sandbox.
