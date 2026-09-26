//! Secure storage contracts and the operating-system keyring implementation.

use async_trait::async_trait;
use keyring::{Entry, Error as KeyringError};
use secrecy::{ExposeSecret, SecretString};
use thiserror::Error;

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

/// Secret Service / native credential-store implementation provided by
/// `keyring`. On Fedora this uses the zbus Secret Service backend.
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
