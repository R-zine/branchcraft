//! Temporary repository fixtures shared by Branchcraft backend tests.

use async_trait::async_trait;
use branchcraft_storage::{SecretStore, StorageError};
use git2::{IndexAddOption, Repository, Signature};
use secrecy::{ExposeSecret, SecretString};
use std::{collections::HashMap, fs, path::Path, sync::Mutex};
use tempfile::TempDir;

/// In-memory credential store for offline authentication tests.
#[derive(Debug, Default)]
pub struct MemorySecretStore {
    secrets: Mutex<HashMap<String, SecretString>>,
}

impl MemorySecretStore {
    /// Returns whether a value is stored without exposing it.
    ///
    /// # Panics
    ///
    /// Panics if a previous test poisoned the in-memory store lock.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.secrets
            .lock()
            .expect("memory secret store lock")
            .contains_key(key)
    }
}

#[async_trait]
impl SecretStore for MemorySecretStore {
    async fn get(&self, key: &str) -> Result<Option<SecretString>, StorageError> {
        Ok(self
            .secrets
            .lock()
            .map_err(|_| StorageError::Unknown)?
            .get(key)
            .cloned())
    }

    async fn set(&self, key: &str, secret: &SecretString) -> Result<(), StorageError> {
        self.secrets
            .lock()
            .map_err(|_| StorageError::Unknown)?
            .insert(key.to_owned(), secret.clone());
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), StorageError> {
        self.secrets
            .lock()
            .map_err(|_| StorageError::Unknown)?
            .remove(key);
        Ok(())
    }
}

/// Verifies the behavior every `SecretStore` implementation must provide.
/// Native adapters can reuse this in an interactive platform test; ordinary CI
/// runs it against `MemorySecretStore` and never opens the user's keyring.
///
/// # Panics
///
/// Panics when the supplied store violates the contract.
pub async fn assert_secret_store_contract(store: &dyn SecretStore) {
    const KEY: &str = "contract-credential";

    store.delete(KEY).await.expect("delete missing credential");
    assert!(
        store
            .get(KEY)
            .await
            .expect("read missing credential")
            .is_none()
    );

    store
        .set(KEY, &SecretString::from("first-secret"))
        .await
        .expect("store credential");
    let first = store
        .get(KEY)
        .await
        .expect("read credential")
        .expect("stored credential");
    assert_eq!(first.expose_secret(), "first-secret");

    store
        .set(KEY, &SecretString::from("replacement-secret"))
        .await
        .expect("replace credential");
    let replacement = store
        .get(KEY)
        .await
        .expect("read replacement")
        .expect("replacement credential");
    assert_eq!(replacement.expose_secret(), "replacement-secret");

    store.delete(KEY).await.expect("delete credential");
    assert!(store.get(KEY).await.expect("read after delete").is_none());
    store
        .delete(KEY)
        .await
        .expect("delete missing credential again");
}

pub struct TestRepository {
    directory: TempDir,
}

impl TestRepository {
    /// Creates an empty temporary Git repository.
    ///
    /// # Panics
    ///
    /// Panics when its isolated directory or Git repository cannot be created.
    #[must_use]
    pub fn empty() -> Self {
        Self::empty_with_prefix("repository-")
    }

    /// Creates an empty repository whose temporary directory has a portable
    /// caller-provided prefix. This is used to exercise spaces and Unicode in
    /// repository roots without hardcoding an operating-system path.
    ///
    /// # Panics
    ///
    /// Panics when its isolated directory or Git repository cannot be created.
    #[must_use]
    pub fn empty_with_prefix(prefix: &str) -> Self {
        let temporary_root = std::env::current_dir()
            .expect("resolve current directory")
            .join("target")
            .join("branchcraft-test-repositories");
        fs::create_dir_all(&temporary_root).expect("create fixture root directory");
        let directory = tempfile::Builder::new()
            .prefix(prefix)
            .tempdir_in(temporary_root)
            .expect("create temporary repository directory");
        let repository =
            Repository::init(directory.path()).expect("initialize temporary repository");
        repository
            .config()
            .expect("open fixture config")
            .set_bool("core.autocrlf", false)
            .expect("isolate fixture line-ending config");
        Self { directory }
    }

    #[must_use]
    pub fn with_initial_commit() -> Self {
        Self::with_initial_commit_and_prefix("repository-")
    }

    #[must_use]
    pub fn with_initial_commit_and_prefix(prefix: &str) -> Self {
        let fixture = Self::empty_with_prefix(prefix);
        fixture.write("README.md", "# Fixture\n");
        fixture.write("delete-me.txt", "remove me\n");
        fixture.stage_all();
        fixture.commit("Initial commit");
        fixture
    }

    /// Sets one repository-local Git configuration value.
    ///
    /// # Panics
    ///
    /// Panics if the repository configuration cannot be updated.
    pub fn set_config(&self, name: &str, value: &str) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        repository
            .config()
            .expect("open fixture config")
            .set_str(name, value)
            .expect("set fixture config");
    }

    #[must_use]
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Writes a UTF-8 fixture file relative to the repository root.
    ///
    /// # Panics
    ///
    /// Panics if the parent directory or file cannot be written.
    pub fn write(&self, relative_path: &str, contents: &str) {
        self.write_bytes(relative_path, contents.as_bytes());
    }

    /// Writes a binary fixture file relative to the repository root.
    ///
    /// # Panics
    ///
    /// Panics if the parent directory or file cannot be written.
    pub fn write_bytes(&self, relative_path: &str, contents: &[u8]) {
        let path = self.path().join(relative_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create fixture parent directory");
        }
        fs::write(path, contents).expect("write fixture file");
    }

    /// Reads a UTF-8 fixture file relative to the repository root.
    ///
    /// # Panics
    ///
    /// Panics if the fixture file cannot be read as UTF-8.
    #[must_use]
    pub fn read(&self, relative_path: &str) -> String {
        fs::read_to_string(self.path().join(relative_path)).expect("read fixture file")
    }

    /// Removes a fixture file.
    ///
    /// # Panics
    ///
    /// Panics if the file cannot be removed.
    pub fn remove(&self, relative_path: &str) {
        fs::remove_file(self.path().join(relative_path)).expect("remove fixture file");
    }

    /// Renames a file and stages both sides of the rename.
    ///
    /// # Panics
    ///
    /// Panics if the file or repository index cannot be updated.
    pub fn rename_and_stage(&self, from: &str, to: &str) {
        let destination = self.path().join(to);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent).expect("create rename destination directory");
        }
        fs::rename(self.path().join(from), destination).expect("rename fixture file");

        let repository = Repository::open(self.path()).expect("open fixture repository");
        let mut index = repository.index().expect("open fixture index");
        index
            .remove_path(Path::new(from))
            .expect("stage removed fixture path");
        index
            .add_path(Path::new(to))
            .expect("stage renamed fixture path");
        index.write().expect("write fixture index");
    }

    /// Detaches HEAD at the current commit.
    ///
    /// # Panics
    ///
    /// Panics if the repository has no current commit or HEAD cannot be set.
    pub fn detach_head(&self) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let commit = repository
            .head()
            .expect("find fixture HEAD")
            .target()
            .expect("resolve fixture HEAD");
        repository
            .set_head_detached(commit)
            .expect("detach fixture HEAD");
    }

    /// Creates a remote-tracking reference at the current commit.
    ///
    /// # Panics
    ///
    /// Panics if `HEAD` cannot be resolved or the reference cannot be written.
    pub fn add_remote_tracking_branch(&self, name: &str) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let target = repository
            .head()
            .expect("find fixture HEAD")
            .target()
            .expect("resolve fixture HEAD");
        repository
            .reference(
                &format!("refs/remotes/{name}"),
                target,
                false,
                "fixture remote-tracking branch",
            )
            .expect("create fixture remote-tracking branch");
    }

    /// Configures a repository-local commit identity.
    ///
    /// # Panics
    ///
    /// Panics if the local Git configuration cannot be updated.
    pub fn configure_identity(&self, name: &str, email: &str) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let mut config = repository.config().expect("open fixture config");
        config.set_str("user.name", name).expect("set fixture name");
        config
            .set_str("user.email", email)
            .expect("set fixture email");
    }

    /// Overrides inherited identity values with invalid local values.
    ///
    /// # Panics
    ///
    /// Panics if the fixture configuration cannot be updated.
    pub fn clear_identity(&self) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let config_path = repository.path().join("config");
        let mut config = git2::Config::open(&config_path).expect("open local fixture config");
        config.set_str("user.name", "").expect("clear fixture name");
        config
            .set_str("user.email", "")
            .expect("clear fixture email");
    }

    /// Returns the complete message of the current fixture commit.
    ///
    /// # Panics
    ///
    /// Panics if `HEAD` does not resolve to a commit with a UTF-8 message.
    #[must_use]
    pub fn head_message(&self) -> String {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        repository
            .head()
            .expect("find fixture HEAD")
            .peel_to_commit()
            .expect("resolve fixture commit")
            .message()
            .expect("read fixture commit message")
            .to_owned()
    }

    /// Stages one fixture path.
    ///
    /// # Panics
    ///
    /// Panics if the repository or index cannot be updated.
    pub fn stage(&self, relative_path: &str) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let mut index = repository.index().expect("open fixture index");
        index
            .add_path(Path::new(relative_path))
            .expect("stage fixture path");
        index.write().expect("write fixture index");
    }

    /// Stages all fixture paths.
    ///
    /// # Panics
    ///
    /// Panics if the repository or index cannot be updated.
    pub fn stage_all(&self) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let mut index = repository.index().expect("open fixture index");
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .expect("stage fixture files");
        index.write().expect("write fixture index");
    }

    /// Creates a commit using the fixture-only identity.
    ///
    /// # Panics
    ///
    /// Panics if the tree, parent, signature, or commit cannot be created.
    pub fn commit(&self, message: &str) {
        let repository = Repository::open(self.path()).expect("open fixture repository");
        let signature = Signature::now("Branchcraft Tests", "tests@branchcraft.local")
            .expect("create fixture signature");
        let mut index = repository.index().expect("open fixture index");
        let tree_id = index.write_tree().expect("write fixture tree");
        let tree = repository.find_tree(tree_id).expect("find fixture tree");
        let parents = repository
            .head()
            .ok()
            .and_then(|head| head.target())
            .and_then(|id| repository.find_commit(id).ok())
            .into_iter()
            .collect::<Vec<_>>();
        let parent_refs = parents.iter().collect::<Vec<_>>();
        repository
            .commit(
                Some("HEAD"),
                &signature,
                &signature,
                message,
                &tree,
                &parent_refs,
            )
            .expect("create fixture commit");
    }
}

#[cfg(test)]
mod tests {
    use super::{MemorySecretStore, assert_secret_store_contract};

    #[tokio::test]
    async fn memory_store_satisfies_secret_store_contract() {
        assert_secret_store_contract(&MemorySecretStore::default()).await;
    }
}
