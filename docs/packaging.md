# Linux packaging

## Application identity and metadata

Tauri owns the Branchcraft product name, version, icons, license, category, and
RPM/DEB bundle configuration in `src-tauri/tauri.conf.json`. AppStream metadata
lives in `packaging/io.github.username.Branchcraft.metainfo.xml` and is included
in RPM builds. Fedora CI builds the RPM and uploads it as an artifact.

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
- [Tauri Flatpak guide](https://v2.tauri.app/distribute/flatpak/)
- [Flatpak manifests](https://docs.flatpak.org/en/latest/manifests.html)
- [Flathub MetaInfo guidelines](https://docs.flathub.org/docs/for-app-authors/metainfo-guidelines)
