//! Secure storage contracts and the operating-system keyring implementation.

use async_trait::async_trait;
use branchcraft_app_core::{
    ApplicationSettings, ColorPalette, RecentRepository, SettingsError, SettingsErrorKind,
    ThemePreference,
};
use keyring::{Entry, Error as KeyringError};
use secrecy::{ExposeSecret, SecretString};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    sync::Mutex,
};
use thiserror::Error;

const MAX_RECENT_REPOSITORIES: usize = 12;
const MAX_SETTINGS_BYTES: u64 = 256 * 1024;

/// Failures from the platform credential store. Error values never contain a
/// credential or platform error payload that could embed one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum StorageError {
    #[error("the secure credential store is unavailable")]
    Unavailable,
    #[error("the secure credential store denied access")]
    AccessDenied,
    #[error("the stored credential is not valid text")]
    Corrupt,
    #[error("the secure credential store operation failed")]
    Unknown,
}

/// Opaque secret persistence used by authentication services.
#[async_trait]
pub trait SecretStore: Send + Sync {
    /// Loads a secret, returning `None` when the key does not exist.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform credential store cannot be read.
    async fn get(&self, key: &str) -> Result<Option<SecretString>, StorageError>;

    /// Stores or replaces a secret under `key`.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform credential store cannot be written.
    async fn set(&self, key: &str, secret: &SecretString) -> Result<(), StorageError>;

    /// Removes a secret. Missing keys are treated as successfully deleted.
    ///
    /// # Errors
    ///
    /// Returns an error when the platform credential store cannot be changed.
    async fn delete(&self, key: &str) -> Result<(), StorageError>;
}

/// Native credential-store implementation provided by `keyring`. Fedora uses
/// Secret Service over zbus; Windows uses Windows Credential Manager.
#[derive(Debug, Clone)]
pub struct KeyringSecretStore {
    service: String,
}

impl KeyringSecretStore {
    #[must_use]
    pub fn new(service: impl Into<String>) -> Self {
        Self {
            service: service.into(),
        }
    }
}

#[async_trait]
impl SecretStore for KeyringSecretStore {
    async fn get(&self, key: &str) -> Result<Option<SecretString>, StorageError> {
        let service = self.service.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let entry = Entry::new(&service, &key).map_err(|error| map_error(&error))?;
            match entry.get_password() {
                Ok(secret) => Ok(Some(SecretString::from(secret))),
                Err(KeyringError::NoEntry) => Ok(None),
                Err(error) => Err(map_error(&error)),
            }
        })
        .await
        .map_err(|_| StorageError::Unknown)?
    }

    async fn set(&self, key: &str, secret: &SecretString) -> Result<(), StorageError> {
        let service = self.service.clone();
        let key = key.to_owned();
        let secret = secret.clone();
        tokio::task::spawn_blocking(move || {
            Entry::new(&service, &key)
                .map_err(|error| map_error(&error))?
                .set_password(secret.expose_secret())
                .map_err(|error| map_error(&error))
        })
        .await
        .map_err(|_| StorageError::Unknown)?
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        let service = self.service.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            let entry = Entry::new(&service, &key).map_err(|error| map_error(&error))?;
            match entry.delete_credential() {
                Ok(()) | Err(KeyringError::NoEntry) => Ok(()),
                Err(error) => Err(map_error(&error)),
            }
        })
        .await
        .map_err(|_| StorageError::Unknown)?
    }
}

fn map_error(error: &KeyringError) -> StorageError {
    match error {
        KeyringError::NoDefaultStore | KeyringError::NotSupportedByStore(_) => {
            StorageError::Unavailable
        }
        KeyringError::NoStorageAccess(_) => StorageError::AccessDenied,
        KeyringError::BadEncoding(_) | KeyringError::BadDataFormat(_, _) => StorageError::Corrupt,
        _ => StorageError::Unknown,
    }
}

/// Persistence contract for non-secret Branchcraft preferences.
pub trait SettingsStore: Send + Sync {
    /// Loads preferences and refreshes recent-repository availability.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when the settings file cannot be read
    /// or decoded.
    fn load(&self) -> Result<ApplicationSettings, SettingsError>;

    /// Saves the selected appearance preference.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when persistence fails.
    fn set_theme(&self, theme: ThemePreference) -> Result<ApplicationSettings, SettingsError>;

    /// Saves the selected color palette.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when persistence fails.
    fn set_palette(&self, palette: ColorPalette) -> Result<ApplicationSettings, SettingsError>;

    /// Moves a repository to the front of the bounded recent list.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when persistence fails.
    fn record_recent_repository(
        &self,
        name: &str,
        path: &Path,
    ) -> Result<ApplicationSettings, SettingsError>;

    /// Removes one path from the recent list.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when persistence fails.
    fn remove_recent_repository(&self, path: &Path) -> Result<ApplicationSettings, SettingsError>;

    /// Removes every recent repository without changing the theme.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when persistence fails.
    fn clear_recent_repositories(&self) -> Result<ApplicationSettings, SettingsError>;

    /// Replaces corrupt or unwanted preferences with safe defaults.
    ///
    /// # Errors
    ///
    /// Returns a safe structured error when persistence fails.
    fn reset(&self) -> Result<ApplicationSettings, SettingsError>;
}

/// Small JSON settings file stored below Tauri's application-config directory.
/// Access is serialized so independent commands cannot lose each other's
/// updates.
#[derive(Debug)]
pub struct JsonSettingsStore {
    path: PathBuf,
    access: Mutex<()>,
}

impl JsonSettingsStore {
    #[must_use]
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            access: Mutex::new(()),
        }
    }

    fn update(
        &self,
        mutation: impl FnOnce(&mut StoredSettings),
    ) -> Result<ApplicationSettings, SettingsError> {
        let _guard = self.access.lock().map_err(|_| unknown_settings_error())?;
        let mut settings = load_stored(&self.path)?;
        mutation(&mut settings);
        normalize(&mut settings);
        write_stored(&self.path, &settings)?;
        Ok(public_settings(settings))
    }
}

impl SettingsStore for JsonSettingsStore {
    fn load(&self) -> Result<ApplicationSettings, SettingsError> {
        let _guard = self.access.lock().map_err(|_| unknown_settings_error())?;
        load_stored(&self.path).map(public_settings)
    }

    fn set_theme(&self, theme: ThemePreference) -> Result<ApplicationSettings, SettingsError> {
        self.update(|settings| settings.theme = theme)
    }

    fn set_palette(&self, palette: ColorPalette) -> Result<ApplicationSettings, SettingsError> {
        self.update(|settings| settings.palette = palette)
    }

    fn record_recent_repository(
        &self,
        name: &str,
        path: &Path,
    ) -> Result<ApplicationSettings, SettingsError> {
        let name = name.trim().to_owned();
        let path = path.to_path_buf();
        self.update(|settings| {
            settings
                .recent_repositories
                .retain(|recent| !same_path(&recent.path, &path));
            settings
                .recent_repositories
                .insert(0, StoredRecentRepository { name, path });
        })
    }

    fn remove_recent_repository(&self, path: &Path) -> Result<ApplicationSettings, SettingsError> {
        let path = path.to_path_buf();
        self.update(|settings| {
            settings
                .recent_repositories
                .retain(|recent| !same_path(&recent.path, &path));
        })
    }

    fn clear_recent_repositories(&self) -> Result<ApplicationSettings, SettingsError> {
        self.update(|settings| settings.recent_repositories.clear())
    }

    fn reset(&self) -> Result<ApplicationSettings, SettingsError> {
        let _guard = self.access.lock().map_err(|_| unknown_settings_error())?;
        let settings = StoredSettings::default();
        write_stored(&self.path, &settings)?;
        Ok(public_settings(settings))
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct StoredSettings {
    theme: ThemePreference,
    palette: ColorPalette,
    recent_repositories: Vec<StoredRecentRepository>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredRecentRepository {
    name: String,
    path: PathBuf,
}

fn load_stored(path: &Path) -> Result<StoredSettings, SettingsError> {
    let metadata = match fs::metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(StoredSettings::default());
        }
        Err(_) => return Err(read_settings_error()),
    };
    if metadata.len() > MAX_SETTINGS_BYTES {
        return Err(corrupt_settings_error());
    }
    let content = fs::read(path).map_err(|_| read_settings_error())?;
    let mut settings =
        serde_json::from_slice::<StoredSettings>(&content).map_err(|_| corrupt_settings_error())?;
    normalize(&mut settings);
    Ok(settings)
}

fn normalize(settings: &mut StoredSettings) {
    settings.recent_repositories.retain(|repository| {
        !repository.name.trim().is_empty() && !repository.path.as_os_str().is_empty()
    });
    let mut unique = Vec::<PathBuf>::new();
    settings.recent_repositories.retain(|repository| {
        if unique.iter().any(|path| same_path(path, &repository.path)) {
            false
        } else {
            unique.push(repository.path.clone());
            true
        }
    });
    settings
        .recent_repositories
        .truncate(MAX_RECENT_REPOSITORIES);
}

fn public_settings(settings: StoredSettings) -> ApplicationSettings {
    ApplicationSettings {
        theme: settings.theme,
        palette: settings.palette,
        recent_repositories: settings
            .recent_repositories
            .into_iter()
            .map(|repository| RecentRepository {
                available: repository.path.is_dir(),
                name: repository.name,
                path: repository.path,
            })
            .collect(),
    }
}

fn write_stored(path: &Path, settings: &StoredSettings) -> Result<(), SettingsError> {
    let parent = path.parent().ok_or_else(write_settings_error)?;
    fs::create_dir_all(parent).map_err(|_| write_settings_error())?;
    let content = serde_json::to_vec_pretty(settings).map_err(|_| write_settings_error())?;

    #[cfg(windows)]
    write_and_sync(path, &content)?;

    #[cfg(not(windows))]
    {
        let temporary = path.with_extension("json.tmp");
        write_and_sync(&temporary, &content)?;
        fs::rename(&temporary, path).map_err(|_| write_settings_error())?;
    }

    Ok(())
}

fn write_and_sync(path: &Path, content: &[u8]) -> Result<(), SettingsError> {
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(path)
        .map_err(|_| write_settings_error())?;
    file.write_all(content)
        .and_then(|()| file.sync_all())
        .map_err(|_| write_settings_error())
}

#[cfg(windows)]
fn same_path(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

#[cfg(not(windows))]
fn same_path(left: &Path, right: &Path) -> bool {
    left == right
}

fn read_settings_error() -> SettingsError {
    SettingsError::new(
        SettingsErrorKind::Read,
        "Branchcraft could not read application settings.",
        Some("Check the application configuration directory, then try again.".to_owned()),
    )
}

fn write_settings_error() -> SettingsError {
    SettingsError::new(
        SettingsErrorKind::Write,
        "Branchcraft could not save application settings.",
        Some("Check available disk space and configuration permissions.".to_owned()),
    )
}

fn corrupt_settings_error() -> SettingsError {
    SettingsError::new(
        SettingsErrorKind::Corrupt,
        "Branchcraft settings are not valid.",
        Some("Reset settings to recover safe defaults.".to_owned()),
    )
}

fn unknown_settings_error() -> SettingsError {
    SettingsError::new(
        SettingsErrorKind::Unknown,
        "Branchcraft settings are temporarily unavailable.",
        Some("Restart Branchcraft, then try again.".to_owned()),
    )
}

#[cfg(test)]
mod settings_tests {
    use super::{JsonSettingsStore, MAX_RECENT_REPOSITORIES, SettingsStore};
    use branchcraft_app_core::{ColorPalette, SettingsErrorKind, ThemePreference};
    use std::fs;
    #[cfg(windows)]
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn store(root: &TempDir) -> JsonSettingsStore {
        JsonSettingsStore::new(root.path().join("config").join("settings.json"))
    }

    #[test]
    fn defaults_then_persists_theme_and_bounded_recents() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        assert_eq!(
            store.load().expect("default settings").theme,
            ThemePreference::System
        );
        assert_eq!(
            store.load().expect("default settings").palette,
            ColorPalette::Branchcraft
        );

        store.set_theme(ThemePreference::Light).expect("save theme");
        store
            .set_palette(ColorPalette::Ocean)
            .expect("save palette");
        for index in 0..MAX_RECENT_REPOSITORIES + 2 {
            let path = root.path().join(format!("repository-{index}"));
            fs::create_dir(&path).expect("repository directory");
            store
                .record_recent_repository(&format!("repository-{index}"), &path)
                .expect("record recent");
        }

        let settings = store.load().expect("stored settings");
        assert_eq!(settings.theme, ThemePreference::Light);
        assert_eq!(settings.palette, ColorPalette::Ocean);
        assert_eq!(settings.recent_repositories.len(), MAX_RECENT_REPOSITORIES);
        assert_eq!(
            settings.recent_repositories[0].name,
            format!("repository-{}", MAX_RECENT_REPOSITORIES + 1)
        );
        assert!(
            settings
                .recent_repositories
                .iter()
                .all(|item| item.available)
        );
    }

    #[test]
    fn deduplicates_and_marks_missing_repositories() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        let path = root.path().join("missing");
        store
            .record_recent_repository("old name", &path)
            .expect("first recent");
        store
            .record_recent_repository("new name", &path)
            .expect("updated recent");

        let settings = store.load().expect("stored settings");
        assert_eq!(settings.recent_repositories.len(), 1);
        assert_eq!(settings.recent_repositories[0].name, "new name");
        assert!(!settings.recent_repositories[0].available);
    }

    #[test]
    fn removes_clears_and_resets_settings() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        let first = root.path().join("first");
        let second = root.path().join("second");
        store
            .record_recent_repository("first", &first)
            .expect("first recent");
        store
            .record_recent_repository("second", &second)
            .expect("second recent");
        let settings = store
            .remove_recent_repository(&first)
            .expect("remove recent");
        assert_eq!(settings.recent_repositories.len(), 1);
        assert_eq!(settings.recent_repositories[0].name, "second");
        assert_eq!(
            store
                .clear_recent_repositories()
                .expect("clear recents")
                .recent_repositories,
            Vec::new()
        );

        store.set_theme(ThemePreference::Dark).expect("dark theme");
        store
            .set_palette(ColorPalette::Monochrome)
            .expect("monochrome palette");
        let settings = store.reset().expect("reset settings");
        assert_eq!(settings.theme, ThemePreference::System);
        assert_eq!(settings.palette, ColorPalette::Branchcraft);
    }

    #[test]
    fn loads_legacy_settings_without_a_palette() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        let path = root.path().join("config").join("settings.json");
        fs::create_dir_all(path.parent().expect("settings parent")).expect("settings parent");
        fs::write(&path, br#"{"theme":"dark","recentRepositories":[]}"#).expect("legacy settings");

        let settings = store.load().expect("load legacy settings");
        assert_eq!(settings.theme, ThemePreference::Dark);
        assert_eq!(settings.palette, ColorPalette::Branchcraft);
    }

    #[test]
    fn reports_corrupt_settings_without_exposing_contents() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        let path = root.path().join("config").join("settings.json");
        fs::create_dir_all(path.parent().expect("settings parent")).expect("settings parent");
        fs::write(&path, b"{not-json").expect("corrupt settings");

        let error = store.load().expect_err("corrupt settings error");
        assert_eq!(error.kind, SettingsErrorKind::Corrupt);
        assert!(!error.summary.contains("not-json"));
        assert_eq!(
            store.reset().expect("reset corrupt settings").theme,
            ThemePreference::System
        );
    }

    #[test]
    fn persists_paths_with_spaces_unicode_and_deep_nesting() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        let path = root
            .path()
            .join("Projects with spaces")
            .join("Гілка")
            .join("nested")
            .join("repository-name_1.2!");
        fs::create_dir_all(&path).expect("repository directory");

        store
            .record_recent_repository("Гілка змін", &path)
            .expect("record portable path");

        let settings = store.load().expect("load portable path");
        assert_eq!(settings.recent_repositories.len(), 1);
        assert_eq!(settings.recent_repositories[0].path, path);
        assert!(settings.recent_repositories[0].available);
    }

    #[cfg(windows)]
    #[test]
    fn deduplicates_normally_case_insensitive_windows_paths() {
        let root = TempDir::new().expect("settings root");
        let store = store(&root);
        let path = root.path().join("MixedCaseRepository");
        let alternate_case = PathBuf::from(path.to_string_lossy().to_ascii_uppercase());

        store
            .record_recent_repository("first", &path)
            .expect("first recent");
        store
            .record_recent_repository("replacement", &alternate_case)
            .expect("replacement recent");

        let settings = store.load().expect("stored settings");
        assert_eq!(settings.recent_repositories.len(), 1);
        assert_eq!(settings.recent_repositories[0].name, "replacement");
    }
}
