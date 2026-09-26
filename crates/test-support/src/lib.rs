//! Temporary repository fixtures shared by Branchcraft backend tests.

use git2::{IndexAddOption, Repository, Signature};
use std::{fs, path::Path};
use tempfile::TempDir;

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
        let temporary_root = std::env::current_dir()
            .expect("resolve current directory")
            .join("target")
            .join("branchcraft-test-repositories");
        fs::create_dir_all(&temporary_root).expect("create fixture root directory");
        let directory = tempfile::Builder::new()
            .prefix("repository-")
            .tempdir_in(temporary_root)
            .expect("create temporary repository directory");
        Repository::init(directory.path()).expect("initialize temporary repository");
        Self { directory }
    }

    #[must_use]
    pub fn with_initial_commit() -> Self {
        let fixture = Self::empty();
        fixture.write("README.md", "# Fixture\n");
        fixture.write("delete-me.txt", "remove me\n");
        fixture.stage_all();
        fixture.commit("Initial commit");
        fixture
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
