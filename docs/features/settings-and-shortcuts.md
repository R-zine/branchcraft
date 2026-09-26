# Settings, themes, and shortcuts

## User outcome

Branchcraft follows the operating-system light/dark preference by default. The
Settings dialog can pin light or dark mode, manages recent repositories, shows
the supported keyboard shortcuts, and contains the application About details.
CSS, Monaco, and the native Tauri window decoration use the same resolved theme.

Successful repository opens and clones move that repository to the front of a
bounded 12-entry recent list. Availability is checked when settings load. A
repository that disappeared from disk is visibly unavailable rather than
causing startup failure, and the entry can be removed or all recents cleared.

## Keyboard shortcuts

| Action | Shortcut |
| --- | --- |
| Open repository | `Ctrl+O` / `Command+O` |
| Clone repository | `Ctrl+Shift+O` / `Command+Shift+O` |
| Refresh repository | `Ctrl+R` / `Command+R` |
| Show changes | `Ctrl+1` / `Command+1` |
| Show history | `Ctrl+2` / `Command+2` |
| Open settings | `Ctrl+,` / `Command+,` |
| Close settings | `Escape` |

Shortcuts are scoped to the Branchcraft window. While the modal settings dialog
is open, background shortcuts are suppressed.

## Ownership and persistence

- `crates/app-core` owns serializable settings and structured settings errors.
- `crates/storage::JsonSettingsStore` owns bounded JSON persistence,
  availability checks, serialized mutations, and corruption recovery.
- `src-tauri/src/commands/settings.rs` moves file access to blocking workers.
- `frontend/src/features/settings` owns theme resolution, the settings dialog,
  shortcuts, recents management, and behavior tests.
- `frontend/src/services/settingsService.ts` is the only raw Tauri boundary and
  synchronizes the native window theme.

The JSON settings file is ordinary application data and contains only theme and
recent-repository metadata. OAuth sessions remain separately protected by the
operating-system keyring. A settings-write failure never causes a successful Git
open or clone to fail.

References:

- [Tauri window theme API](https://v2.tauri.app/reference/javascript/api/namespacewindow/)
