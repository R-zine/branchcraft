# Packaging

## Application identity and metadata

Tauri owns the Branchcraft product name, version, icons, license, category, and
native bundle configuration in `src-tauri/tauri.conf.json`. AppStream metadata
lives in `packaging/io.github.username.Branchcraft.metainfo.xml` and is included
in RPM builds. The Windows bundle uses the existing Branchcraft ICO and Tauri's
NSIS metadata. CI builds and uploads both primary artifacts on their native
operating systems.

`io.github.username.Branchcraft` is intentionally still a configurable
placeholder because the final GitHub owner is unknown. Before a public release,
replace `username` consistently in:

- `src-tauri/tauri.conf.json`;
- the AppStream MetaInfo filename and contents;
- the Flatpak manifest filename, ID, and README commands;
- the workspace repository URL.

Then validate the MetaInfo and desktop export and sign published artifacts.

## RPM

On Fedora with the prerequisites from [development.md](development.md):

```bash
cargo install tauri-cli --version '^2.11' --locked
cargo tauri build --config src-tauri/tauri.conf.json --bundles rpm
```

The optimized package is written below `target/release/bundle/rpm`. It includes
the Branchcraft icon, generated desktop entry, license metadata, and AppStream
MetaInfo. Tauri documents Linux RPM bundle customization in its
[configuration reference](https://v2.tauri.app/reference/config/).

## Windows NSIS

On a Windows 11 x64 development machine with the prerequisites from
[development.md](development.md):

```powershell
cargo install tauri-cli --version '^2.11' --locked
cargo tauri build --config src-tauri/tauri.conf.json --bundles nsis
```

The optimized installer is written below `target\release\bundle\nsis`, normally
as `Branchcraft_<version>_x64-setup.exe`. It installs for the current user by
default, creates a Start Menu shortcut, registers a normal uninstall entry, and
uses Tauri's WebView2 bootstrapper behavior. CI performs this build on a native
Windows runner and uploads the `.exe`.

The installer is intentionally unsigned. Windows can display an
unknown-publisher or SmartScreen warning; that is acceptable for this
open-source distribution. Branchcraft does not add Microsoft Store, MSIX,
commercial certificate, EV signing, updater, or enterprise deployment
infrastructure.

GitHub Releases can publish the naturally generated files directly or rename
them consistently to forms such as:

```text
Branchcraft-<version>-windows-x64-setup.exe
Branchcraft-<version>-linux-x86_64.rpm
```

Renaming a release upload does not change embedded product/version metadata.

## Flatpak

`packaging/flatpak/io.github.username.Branchcraft.yml` is a practical local
manifest targeting the supported GNOME 50 runtime. It packages the Tauri Debian
release binary, exports ID-prefixed desktop metadata through Flatpak's rename
rules, and grants only Wayland, graphics, network, desktop-portal, and Secret
Service access. It deliberately does not grant blanket home-directory access.

Follow `packaging/flatpak/README.md` to build and stage a local bundle. A Flathub
submission still requires a finalized application ID, hosted immutable release
sources, checksums, screenshots, and Flathub review. The committed manifest is
therefore production-oriented local packaging, not a claim of publication.

References:

- [Tauri distribution](https://v2.tauri.app/distribute/)
- [Tauri Windows installers](https://v2.tauri.app/distribute/windows-installer/)
- [Tauri Flatpak guide](https://v2.tauri.app/distribute/flatpak/)
- [Flatpak manifests](https://docs.flatpak.org/en/latest/manifests.html)
- [Flathub MetaInfo guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines)
