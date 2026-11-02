use super::{CloneSourceKind, remote::add_github_credentials};
use branchcraft_app_core::{CloneProgress, CloneProgressPhase, GitError, GitErrorKind};
use git2::{
    ErrorClass, ErrorCode, FetchOptions, RemoteCallbacks, RemoteRedirect,
    build::{CheckoutBuilder, CloneLocal, RepoBuilder},
};
use secrecy::SecretString;
use std::{
    cell::RefCell,
    fs,
    path::{Component, Path, PathBuf},
};
use url::Url;

type ProgressCallback<'a> = dyn FnMut(CloneProgress) + 'a;

struct CloneSource {
    kind: CloneSourceKind,
    value: String,
    repository_name: String,
}

pub(super) fn source_kind(source: &str) -> Result<CloneSourceKind, GitError> {
    validate_source(source).map(|source| source.kind)
}

pub(super) fn clone_repository(
    source: &str,
    destination_parent: &Path,
    token: Option<&SecretString>,
    progress: &mut ProgressCallback<'_>,
) -> Result<PathBuf, GitError> {
    let source = validate_source(source)?;
    let parent = validate_destination_parent(destination_parent)?;
    let destination = parent.join(&source.repository_name);
    if destination.exists() {
        return Err(destination_exists());
    }

    emit(progress, CloneProgressPhase::Connecting, 0, 0, 0);
    let progress = RefCell::new(progress);
    let mut callbacks = RemoteCallbacks::new();
    if source.kind == CloneSourceKind::GitHubHttps
        && let Some(token) = token
    {
        add_github_credentials(&mut callbacks, token);
    }
    callbacks.transfer_progress(|stats| {
        emit(
            &mut **progress.borrow_mut(),
            CloneProgressPhase::Receiving,
            count(stats.received_objects().max(stats.indexed_objects())),
            count(stats.total_objects()),
            count(stats.received_bytes()),
        );
        true
    });

    let mut fetch = FetchOptions::new();
    fetch
        .remote_callbacks(callbacks)
        .follow_redirects(RemoteRedirect::None);
    let mut checkout = CheckoutBuilder::new();
    checkout.safe().progress(|_, current, total| {
        emit(
            &mut **progress.borrow_mut(),
            CloneProgressPhase::Checkout,
            count(current),
            count(total),
            0,
        );
    });
    let mut builder = RepoBuilder::new();
    builder
        .fetch_options(fetch)
        .with_checkout(checkout)
        .clone_local(CloneLocal::None);
    builder
        .clone(&source.value, &destination)
        .map_err(|error| map_clone_error(&error, source.kind))?;
    emit(
        &mut **progress.borrow_mut(),
        CloneProgressPhase::Completed,
        1,
        1,
        0,
    );
    Ok(destination)
}

fn validate_source(source: &str) -> Result<CloneSource, GitError> {
    let source = source.trim();
    if source.is_empty() || source.contains('\0') {
        return Err(invalid_source());
    }

    let path = Path::new(source);
    if path.is_absolute() && !source.starts_with("//") && !source.starts_with(r"\\") {
        return Ok(CloneSource {
            kind: CloneSourceKind::Local,
            value: source.to_owned(),
            repository_name: repository_name_from_path(path)?,
        });
    }

    let url = Url::parse(source).map_err(|_| invalid_source())?;
    let has_credentials = !url.username().is_empty() || url.password().is_some();
    if has_credentials || url.query().is_some() || url.fragment().is_some() {
        return Err(invalid_source());
    }

    if url.scheme() == "file" && url.host_str().is_none() {
        let path = url.to_file_path().map_err(|()| invalid_source())?;
        return Ok(CloneSource {
            kind: CloneSourceKind::Local,
            value: url.to_string(),
            repository_name: repository_name_from_path(&path)?,
        });
    }

    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.port_or_known_default() != Some(443)
    {
        return Err(invalid_source());
    }
    let repository_name = url
        .path_segments()
        .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
        .map(repository_name)
        .transpose()?
        .ok_or_else(invalid_source)?;
    let kind = if url
        .host_str()
        .is_some_and(|host| host.eq_ignore_ascii_case("github.com"))
    {
        CloneSourceKind::GitHubHttps
    } else {
        CloneSourceKind::AnonymousHttps
    };
    Ok(CloneSource {
        kind,
        value: url.to_string(),
        repository_name,
    })
}

fn repository_name_from_path(path: &Path) -> Result<String, GitError> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(repository_name)
        .transpose()?
        .ok_or_else(invalid_source)
}

fn repository_name(name: &str) -> Result<String, GitError> {
    let name = name.strip_suffix(".git").unwrap_or(name);
    let path = Path::new(name);
    if name.is_empty()
        || name == "."
        || name == ".."
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        Err(invalid_source())
    } else {
        Ok(name.to_owned())
    }
}

fn validate_destination_parent(path: &Path) -> Result<PathBuf, GitError> {
    if path.as_os_str().is_empty() {
        return Err(invalid_destination());
    }
    let parent = fs::canonicalize(path).map_err(|error| match error.kind() {
        std::io::ErrorKind::PermissionDenied => GitError::new(
            GitErrorKind::PermissionDenied,
            "Branchcraft cannot write to the selected destination.",
            Some("Choose a folder you can write to, then try again.".to_owned()),
        ),
        _ => invalid_destination(),
    })?;
    if parent.is_dir() {
        Ok(parent)
    } else {
        Err(invalid_destination())
    }
}

fn invalid_source() -> GitError {
    GitError::new(
        GitErrorKind::InvalidRemoteUrl,
        "Enter a valid repository URL.",
        Some(
            "Use a credential-free HTTPS URL, a hostless file URL, or an absolute local path."
                .to_owned(),
        ),
    )
}

fn invalid_destination() -> GitError {
    GitError::new(
        GitErrorKind::InvalidPath,
        "The clone destination is not available.",
        Some("Choose an existing writable parent folder and try again.".to_owned()),
    )
}

fn destination_exists() -> GitError {
    GitError::new(
        GitErrorKind::DestinationExists,
        "A folder with this repository name already exists.",
        Some("Choose another parent folder or rename the existing folder.".to_owned()),
    )
}

fn map_clone_error(error: &git2::Error, kind: CloneSourceKind) -> GitError {
    if error.code() == ErrorCode::Exists {
        destination_exists()
    } else if error.code() == ErrorCode::Auth {
        match kind {
            CloneSourceKind::GitHubHttps => GitError::new(
                GitErrorKind::Authentication,
                "GitHub authentication is required to clone this repository.",
                Some("Sign in to GitHub, then try again.".to_owned()),
            ),
            CloneSourceKind::AnonymousHttps | CloneSourceKind::Local => GitError::new(
                GitErrorKind::UnsupportedRemote,
                "This repository requires unsupported credentials.",
                Some("Use a public HTTPS URL or a GitHub repository.".to_owned()),
            ),
        }
    } else if error.code() == ErrorCode::NotFound {
        GitError::new(
            GitErrorKind::RepositoryNotFound,
            "The source repository could not be found.",
            Some("Check the repository URL and your access, then try again.".to_owned()),
        )
    } else if error.code() == ErrorCode::Certificate {
        GitError::new(
            GitErrorKind::Network,
            "The repository's secure connection could not be verified.",
            Some("Check the repository URL and system certificates.".to_owned()),
        )
    } else if matches!(
        error.class(),
        ErrorClass::Net | ErrorClass::Http | ErrorClass::Ssl | ErrorClass::Ssh
    ) || error.code() == ErrorCode::Timeout
    {
        GitError::new(
            GitErrorKind::Network,
            "The source repository could not be reached.",
            Some("Check your network connection and repository URL, then try again.".to_owned()),
        )
    } else if error.class() == ErrorClass::Os {
        GitError::new(
            GitErrorKind::PermissionDenied,
            "Branchcraft could not create the cloned repository.",
            Some("Check destination permissions and available disk space.".to_owned()),
        )
    } else {
        GitError::new(
            GitErrorKind::Unknown,
            "Branchcraft could not clone the repository.",
            Some(
                "A partial destination folder may remain. Remove it before trying again."
                    .to_owned(),
            ),
        )
    }
}

fn emit(
    progress: &mut ProgressCallback<'_>,
    phase: CloneProgressPhase,
    current: u64,
    total: u64,
    bytes: u64,
) {
    progress(CloneProgress {
        phase,
        current,
        total,
        bytes,
    });
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LibGitService;
    use branchcraft_test_support::TestRepository;
    use tempfile::TempDir;

    fn clone_parent() -> TempDir {
        let root = std::env::current_dir()
            .expect("current directory")
            .join("target")
            .join("branchcraft-clone-tests");
        fs::create_dir_all(&root).expect("clone fixture root");
        tempfile::Builder::new()
            .prefix("destination-")
            .tempdir_in(root)
            .expect("clone destination parent")
    }

    #[test]
    fn validates_secure_clone_sources() {
        assert_eq!(
            source_kind("https://github.com/example/project.git").expect("GitHub source"),
            CloneSourceKind::GitHubHttps
        );
        assert_eq!(
            source_kind("https://git.example.com/example/project.git")
                .expect("anonymous HTTPS source"),
            CloneSourceKind::AnonymousHttps
        );
        for invalid in [
            "http://github.com/example/project.git",
            "git@github.com:example/project.git",
            "https://token@github.com/example/project.git",
            "https://github.com/example/project.git?token=secret",
        ] {
            assert_eq!(
                source_kind(invalid).expect_err("unsafe source").kind,
                GitErrorKind::InvalidRemoteUrl
            );
        }
    }

    #[test]
    fn clones_a_local_repository_with_progress() {
        let source =
            TestRepository::with_initial_commit_and_prefix("clone source with spaces Гілка-");
        let parent = clone_parent();
        let mut events = Vec::new();

        let destination = clone_repository(
            source.path().to_string_lossy().as_ref(),
            parent.path(),
            None,
            &mut |event| events.push(event),
        )
        .expect("local clone");
        let snapshot = LibGitService::snapshot(&destination).expect("cloned snapshot");

        assert_eq!(snapshot.files, Vec::new());
        assert!(destination.join("README.md").is_file());
        assert!(
            events
                .iter()
                .any(|event| event.phase == CloneProgressPhase::Receiving)
        );
        assert_eq!(
            events.last().map(|event| event.phase),
            Some(CloneProgressPhase::Completed)
        );
    }

    #[test]
    fn refuses_to_overwrite_an_existing_destination() {
        let source = TestRepository::with_initial_commit();
        let parent = clone_parent();
        let name = source.path().file_name().expect("source name");
        fs::create_dir(parent.path().join(name)).expect("existing destination");

        let error = clone_repository(
            source.path().to_string_lossy().as_ref(),
            parent.path(),
            None,
            &mut |_| {},
        )
        .expect_err("destination collision");

        assert_eq!(error.kind, GitErrorKind::DestinationExists);
    }
}
