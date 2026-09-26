# Branchcraft Flatpak

The manifest packages the same release binary produced by Tauri and targets the
GNOME 50 runtime. The desktop, icon, and MetaInfo exports are renamed to the
Flatpak application ID by `flatpak-builder`.

Before publishing, replace `username` in the application ID here, in the root
MetaInfo file, and in `src-tauri/tauri.conf.json` with the final GitHub owner.

Build and stage the Debian bundle from the repository root:

```powershell
cargo tauri build --config src-tauri/tauri.conf.json --bundles deb
./packaging/flatpak/prepare.ps1
```

Then, from `packaging/flatpak`, install the runtime and build locally:

```bash
flatpak install --user flathub org.gnome.Platform//50 org.gnome.Sdk//50
flatpak-builder --force-clean --user --install-deps-from=flathub \
  --repo=repository build-dir io.github.username.Branchcraft.yml
flatpak build-bundle repository Branchcraft.flatpak \
  io.github.username.Branchcraft
```

The manifest grants Wayland, graphics, network, desktop-portal, and Secret
Service access. It does not grant blanket home-directory access; native file
chooser portals provide access to repositories explicitly selected by the user.
