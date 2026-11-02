use super::{LibGitService, canonical_directory, changed_files, map_open_error};
use branchcraft_app_core::{
    GitError, GitErrorKind, RemoteInfo, RemoteKind, RemoteOperation, RemoteOperationResult,
    RemoteOutcome, RemoteProgress, RemoteProgressPhase,
};
use git2::{
    Cred, CredentialType, ErrorClass, ErrorCode, FetchOptions, PushOptions, RemoteCallbacks,
    RemoteRedirect, Repository, build::CheckoutBuilder,
};
use secrecy::{ExposeSecret, SecretString};
use std::{cell::Cell, path::Path};
use url::Url;

type ProgressCallback<'a> = dyn FnMut(RemoteProgress) + 'a;

struct BranchContext {
    local_ref: String,
    branch_name: String,
    upstream_ref: String,
    push_ref: String,
}

pub(super) fn primary_remote(path: &Path) -> Result<Option<RemoteInfo>, GitError> {
    let repository = open_repository(path)?;
    let Some(name) = select_remote_name(&repository)? else {
        return Ok(None);
    };
    remote_info(&repository, &name).map(Some)
}

pub(super) fn fetch(
    path: &Path,
    token: Option<&SecretString>,
    progress: &mut ProgressCallback<'_>,
) -> Result<RemoteOperationResult, GitError> {
    let repository = open_repository(path)?;
    let name = require_remote_name(&repository)?;
    let changed = perform_fetch(&repository, &name, token, RemoteOperation::Fetch, progress)?;
    emit(
        progress,
        RemoteOperation::Fetch,
        RemoteProgressPhase::Completed,
        &name,
        1,
        1,
        0,
    );
    operation_result(
        path,
        &repository,
        &name,
        if changed {
            RemoteOutcome::Fetched
        } else {
            RemoteOutcome::UpToDate
        },
    )
}

pub(super) fn pull(
    path: &Path,
    token: Option<&SecretString>,
    progress: &mut ProgressCallback<'_>,
) -> Result<RemoteOperationResult, GitError> {
    let repository = open_repository(path)?;
    let name = require_remote_name(&repository)?;
    perform_fetch(&repository, &name, token, RemoteOperation::Pull, progress)?;

    let branch = branch_context(&repository, &name)?;
    let local_id = repository
        .find_reference(&branch.local_ref)
        .and_then(|reference| {
            reference
                .target()
                .ok_or_else(|| git2::Error::from_str("local branch has no target"))
        })
        .map_err(|error| map_remote_error(&error))?;
    let upstream_id = repository
        .find_reference(&branch.upstream_ref)
        .and_then(|reference| {
            reference
                .target()
                .ok_or_else(|| git2::Error::from_str("upstream branch has no target"))
        })
        .map_err(|_| missing_upstream(&name, &branch.branch_name))?;
    let (ahead, behind) = repository
        .graph_ahead_behind(local_id, upstream_id)
        .map_err(|error| map_remote_error(&error))?;

    let outcome = if behind == 0 {
        RemoteOutcome::UpToDate
    } else {
        ensure_clean(&repository)?;
        if ahead == 0 {
            fast_forward(&repository, &branch.local_ref, upstream_id)?;
            RemoteOutcome::FastForwarded
        } else {
            emit(
                progress,
                RemoteOperation::Pull,
                RemoteProgressPhase::Merging,
                &name,
                0,
                1,
                0,
            );
            merge_diverged(
                &repository,
                &name,
                &branch.local_ref,
                &branch.branch_name,
                local_id,
                upstream_id,
            )?;
            RemoteOutcome::Merged
        }
    };

    emit(
        progress,
        RemoteOperation::Pull,
        RemoteProgressPhase::Completed,
        &name,
        1,
        1,
        0,
    );
    operation_result(path, &repository, &name, outcome)
}

pub(super) fn push(
    path: &Path,
    token: Option<&SecretString>,
    progress: &mut ProgressCallback<'_>,
) -> Result<RemoteOperationResult, GitError> {
    let repository = open_repository(path)?;
    let name = require_remote_name(&repository)?;
    let info = remote_info(&repository, &name)?;
    validate_transport(&info, token)?;
    let branch = branch_context(&repository, &name)?;
    let local_id = repository
        .find_reference(&branch.local_ref)
        .and_then(|reference| {
            reference
                .target()
                .ok_or_else(|| git2::Error::from_str("local branch has no target"))
        })
        .map_err(|error| map_remote_error(&error))?;
    let was_ahead = info.ahead;

    emit(
        progress,
        RemoteOperation::Push,
        RemoteProgressPhase::Connecting,
        &name,
        0,
        0,
        0,
    );

    perform_push(&repository, &name, info.kind, token, &branch, progress)?;

    update_tracking_reference(&repository, &branch.upstream_ref, local_id)?;
    emit(
        progress,
        RemoteOperation::Push,
        RemoteProgressPhase::Updating,
        &name,
        1,
        1,
        0,
    );
    emit(
        progress,
        RemoteOperation::Push,
        RemoteProgressPhase::Completed,
        &name,
        1,
        1,
        0,
    );
    operation_result(
        path,
        &repository,
        &name,
        if was_ahead == 0 && info.tracking {
            RemoteOutcome::UpToDate
        } else {
            RemoteOutcome::Pushed
        },
    )
}

fn perform_push(
    repository: &Repository,
    name: &str,
    kind: RemoteKind,
    token: Option<&SecretString>,
    branch: &BranchContext,
    progress: &mut ProgressCallback<'_>,
) -> Result<(), GitError> {
    let rejection = std::cell::RefCell::new(None::<PushRejection>);
    {
        let mut callbacks = RemoteCallbacks::new();
        add_credentials(&mut callbacks, kind, token);
        callbacks.push_transfer_progress(|current, total, bytes| {
            emit(
                progress,
                RemoteOperation::Push,
                RemoteProgressPhase::Transferring,
                name,
                count(current),
                count(total),
                count(bytes),
            );
        });
        callbacks.push_update_reference(|_, status| {
            if let Some(status) = status {
                let normalized = status.to_ascii_lowercase();
                *rejection.borrow_mut() = Some(
                    if normalized.contains("non-fast-forward") || normalized.contains("fetch first")
                    {
                        PushRejection::NonFastForward
                    } else {
                        PushRejection::Rejected
                    },
                );
            }
            Ok(())
        });

        let mut options = PushOptions::new();
        options
            .remote_callbacks(callbacks)
            .follow_redirects(RemoteRedirect::None);
        let mut remote = repository
            .find_remote(name)
            .map_err(|error| map_remote_error(&error))?;
        let refspec = format!("{}:{}", branch.local_ref, branch.push_ref);
        remote
            .push(&[refspec], Some(&mut options))
            .map_err(|error| map_remote_error(&error))?;
    }

    match rejection.into_inner() {
        Some(PushRejection::NonFastForward) => Err(non_fast_forward()),
        Some(PushRejection::Rejected) => Err(GitError::new(
            GitErrorKind::PushRejected,
            "The remote rejected the push.",
            Some("Check repository permissions and branch protection, then try again.".to_owned()),
        )),
        None => Ok(()),
    }
}

fn perform_fetch(
    repository: &Repository,
    name: &str,
    token: Option<&SecretString>,
    operation: RemoteOperation,
    progress: &mut ProgressCallback<'_>,
) -> Result<bool, GitError> {
    let info = remote_info(repository, name)?;
    validate_transport(&info, token)?;
    emit(
        progress,
        operation,
        RemoteProgressPhase::Connecting,
        name,
        0,
        0,
        0,
    );

    let updated = Cell::new(false);
    let received_objects;
    {
        let mut callbacks = RemoteCallbacks::new();
        add_credentials(&mut callbacks, info.kind, token);
        callbacks.transfer_progress(|stats| {
            emit(
                progress,
                operation,
                RemoteProgressPhase::Transferring,
                name,
                count(stats.received_objects().max(stats.indexed_objects())),
                count(stats.total_objects()),
                count(stats.received_bytes()),
            );
            true
        });
        callbacks.update_tips(|_, old, new| {
            if old != new {
                updated.set(true);
            }
            true
        });

        let mut options = FetchOptions::new();
        options
            .remote_callbacks(callbacks)
            .follow_redirects(RemoteRedirect::None);
        let mut remote = repository
            .find_remote(name)
            .map_err(|error| map_remote_error(&error))?;
        remote
            .fetch(
                &[] as &[&str],
                Some(&mut options),
                Some("Branchcraft fetch"),
            )
            .map_err(|error| map_remote_error(&error))?;
        received_objects = remote.stats().received_objects();
    }

    emit(
        progress,
        operation,
        RemoteProgressPhase::Updating,
        name,
        1,
        1,
        0,
    );
    Ok(updated.get() || received_objects > 0)
}

fn add_credentials<'a>(
    callbacks: &mut RemoteCallbacks<'a>,
    kind: RemoteKind,
    token: Option<&'a SecretString>,
) {
    if kind != RemoteKind::GitHubHttps {
        return;
    }
    let Some(token) = token else {
        return;
    };
    add_github_credentials(callbacks, token);
}

pub(super) fn add_github_credentials<'a>(
    callbacks: &mut RemoteCallbacks<'a>,
    token: &'a SecretString,
) {
    callbacks.credentials(move |url, _, allowed| {
        if !is_safe_github_url(url) {
            return Err(git2::Error::from_str("credential host mismatch"));
        }
        if allowed.contains(CredentialType::USER_PASS_PLAINTEXT) {
            Cred::userpass_plaintext("x-access-token", token.expose_secret())
        } else if allowed.contains(CredentialType::USERNAME) {
            Cred::username("x-access-token")
        } else {
            Err(git2::Error::from_str("unsupported credential type"))
        }
    });
}

fn validate_transport(info: &RemoteInfo, token: Option<&SecretString>) -> Result<(), GitError> {
    match info.kind {
        RemoteKind::GitHubHttps if token.is_none() => Err(GitError::new(
            GitErrorKind::Authentication,
            "Sign in with GitHub to use this remote.",
            Some("Use the account menu, then try the operation again.".to_owned()),
        )),
        RemoteKind::GitHubHttps | RemoteKind::Local => Ok(()),
        RemoteKind::Unsupported => Err(GitError::new(
            GitErrorKind::UnsupportedRemote,
            "This remote cannot be used securely by Branchcraft.",
            Some(
                "Use a credential-free https://github.com/owner/repository.git URL. SSH and other hosts are not enabled."
                    .to_owned(),
            ),
        )),
    }
}

fn open_repository(path: &Path) -> Result<Repository, GitError> {
    let path = canonical_directory(path)?;
    Repository::open(path).map_err(|error| map_open_error(&error))
}

fn require_remote_name(repository: &Repository) -> Result<String, GitError> {
    select_remote_name(repository)?.ok_or_else(|| {
        GitError::new(
            GitErrorKind::NoRemote,
            "This repository has no remote.",
            Some("Add a GitHub HTTPS remote before using fetch, pull, or push.".to_owned()),
        )
    })
}

fn select_remote_name(repository: &Repository) -> Result<Option<String>, GitError> {
    if let Ok(head) = repository.head()
        && head.is_branch()
        && let Ok(local_ref) = head.name()
        && let Ok(remote) = repository.branch_upstream_remote(local_ref)
        && let Ok(remote) = remote.as_str()
        && repository.find_remote(remote).is_ok()
    {
        return Ok(Some(remote.to_owned()));
    }

    let remotes = repository
        .remotes()
        .map_err(|error| map_remote_error(&error))?;
    let mut names = remotes
        .iter()
        .filter_map(Result::ok)
        .flatten()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    names.sort_unstable();
    if names.iter().any(|name| name == "origin") {
        Ok(Some("origin".to_owned()))
    } else {
        Ok(names.into_iter().next())
    }
}

fn remote_info(repository: &Repository, name: &str) -> Result<RemoteInfo, GitError> {
    let remote = repository
        .find_remote(name)
        .map_err(|error| map_remote_error(&error))?;
    let raw_url = remote.url().map_err(|_| {
        GitError::new(
            GitErrorKind::UnsupportedRemote,
            "The remote has no usable URL.",
            Some("Configure a GitHub HTTPS URL and try again.".to_owned()),
        )
    })?;
    let (kind, display_url) = classify_remote_url(raw_url);
    let (tracking, ahead, behind) = tracking_counts(repository, name)?;
    Ok(RemoteInfo {
        name: name.to_owned(),
        display_url,
        kind,
        tracking,
        ahead: count(ahead),
        behind: count(behind),
    })
}

fn classify_remote_url(raw_url: &str) -> (RemoteKind, String) {
    if Path::new(raw_url).is_absolute() && !raw_url.starts_with("//") && !raw_url.starts_with(r"\\")
    {
        return (RemoteKind::Local, "Local repository".to_owned());
    }
    if let Ok(mut parsed) = Url::parse(raw_url) {
        if parsed.scheme() == "file" && parsed.host_str().is_none() {
            return (RemoteKind::Local, "Local repository".to_owned());
        }
        let had_credentials = !parsed.username().is_empty() || parsed.password().is_some();
        let host = parsed.host_str().unwrap_or_default().to_owned();
        let secure_github = parsed.scheme() == "https"
            && host.eq_ignore_ascii_case("github.com")
            && parsed.port_or_known_default() == Some(443)
            && parsed.query().is_none()
            && parsed.fragment().is_none()
            && !had_credentials;
        let _ = parsed.set_username("");
        let _ = parsed.set_password(None);
        parsed.set_query(None);
        parsed.set_fragment(None);
        let display = parsed.to_string();
        return (
            if secure_github {
                RemoteKind::GitHubHttps
            } else {
                RemoteKind::Unsupported
            },
            display,
        );
    }

    let display = raw_url.rsplit_once('@').map_or_else(
        || "Unsupported remote".to_owned(),
        |(_, safe)| safe.to_owned(),
    );
    (RemoteKind::Unsupported, display)
}

pub(super) fn is_safe_github_url(raw_url: &str) -> bool {
    let Ok(parsed) = Url::parse(raw_url) else {
        return false;
    };
    parsed.scheme() == "https"
        && parsed
            .host_str()
            .is_some_and(|host| host.eq_ignore_ascii_case("github.com"))
        && parsed.port_or_known_default() == Some(443)
        && parsed.username().is_empty()
        && parsed.password().is_none()
        && parsed.query().is_none()
        && parsed.fragment().is_none()
}

fn tracking_counts(
    repository: &Repository,
    remote_name: &str,
) -> Result<(bool, usize, usize), GitError> {
    let Ok(head) = repository.head() else {
        return Ok((false, 0, 0));
    };
    if !head.is_branch() {
        return Ok((false, 0, 0));
    }
    let Ok(local_ref) = head.name() else {
        return Ok((false, 0, 0));
    };
    let Some(local_id) = head.target() else {
        return Ok((false, 0, 0));
    };
    let branch_name = local_ref.strip_prefix("refs/heads/").unwrap_or(local_ref);
    let upstream_ref = repository
        .branch_upstream_name(local_ref)
        .ok()
        .and_then(|name| name.as_str().ok().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("refs/remotes/{remote_name}/{branch_name}"));
    let Ok(upstream) = repository.find_reference(&upstream_ref) else {
        return Ok((false, 0, 0));
    };
    let Some(upstream_id) = upstream.target() else {
        return Ok((false, 0, 0));
    };
    let (ahead, behind) = repository
        .graph_ahead_behind(local_id, upstream_id)
        .map_err(|error| map_remote_error(&error))?;
    Ok((true, ahead, behind))
}

fn branch_context(repository: &Repository, remote_name: &str) -> Result<BranchContext, GitError> {
    let head = repository.head().map_err(|error| {
        if error.code() == ErrorCode::UnbornBranch {
            detached_head("Create the first commit before using remote operations.")
        } else {
            map_remote_error(&error)
        }
    })?;
    if !head.is_branch() {
        return Err(detached_head(
            "Check out a local branch before pulling or pushing.",
        ));
    }
    let local_ref = head
        .name()
        .map_err(|_| detached_head("Check out a named local branch before pulling or pushing."))?;
    let branch_name = local_ref
        .strip_prefix("refs/heads/")
        .ok_or_else(|| detached_head("The current branch name could not be resolved."))?;
    let upstream_ref = repository
        .branch_upstream_name(local_ref)
        .ok()
        .and_then(|name| name.as_str().ok().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("refs/remotes/{remote_name}/{branch_name}"));
    let push_ref = repository
        .branch_upstream_merge(local_ref)
        .ok()
        .and_then(|name| name.as_str().ok().map(ToOwned::to_owned))
        .unwrap_or_else(|| format!("refs/heads/{branch_name}"));
    Ok(BranchContext {
        local_ref: local_ref.to_owned(),
        branch_name: branch_name.to_owned(),
        upstream_ref,
        push_ref,
    })
}

fn ensure_clean(repository: &Repository) -> Result<(), GitError> {
    if changed_files(repository)?.is_empty() {
        Ok(())
    } else {
        Err(GitError::new(
            GitErrorKind::DirtyWorkingTree,
            "Pull stopped because the repository has local changes.",
            Some("Commit or discard all changes before pulling remote commits.".to_owned()),
        ))
    }
}

fn fast_forward(
    repository: &Repository,
    local_ref: &str,
    target: git2::Oid,
) -> Result<(), GitError> {
    let object = repository
        .find_object(target, None)
        .map_err(|error| map_remote_error(&error))?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    repository
        .checkout_tree(&object, Some(&mut checkout))
        .map_err(|_| {
            GitError::new(
                GitErrorKind::DirtyWorkingTree,
                "Pull could not update the working tree safely.",
                Some("Refresh the repository and check for local changes.".to_owned()),
            )
        })?;
    repository
        .find_reference(local_ref)
        .and_then(|mut reference| {
            reference.set_target(target, "Branchcraft pull: fast-forward")?;
            Ok(())
        })
        .map_err(|error| map_remote_error(&error))
}

fn merge_diverged(
    repository: &Repository,
    remote_name: &str,
    local_ref: &str,
    branch_name: &str,
    local_id: git2::Oid,
    upstream_id: git2::Oid,
) -> Result<(), GitError> {
    let local = repository
        .find_commit(local_id)
        .map_err(|error| map_remote_error(&error))?;
    let upstream = repository
        .find_commit(upstream_id)
        .map_err(|error| map_remote_error(&error))?;
    let mut merge_index = repository
        .merge_commits(&local, &upstream, None)
        .map_err(|error| map_remote_error(&error))?;
    if merge_index.has_conflicts() {
        return Err(GitError::new(
            GitErrorKind::Conflict,
            "Pull would create merge conflicts.",
            Some(
                "No working files were changed. Merge or rebase the remote branch manually, then try again."
                    .to_owned(),
            ),
        ));
    }
    let signature = repository.signature().map_err(|_| {
        GitError::new(
            GitErrorKind::MissingIdentity,
            "Git needs your name and email to create the merge commit.",
            Some("Set user.name and user.email in Git configuration, then try again.".to_owned()),
        )
    })?;
    if !signature.name().is_ok_and(|name| !name.is_empty())
        || !signature.email().is_ok_and(|email| !email.is_empty())
    {
        return Err(GitError::new(
            GitErrorKind::MissingIdentity,
            "Git needs your name and email to create the merge commit.",
            Some("Set user.name and user.email in Git configuration, then try again.".to_owned()),
        ));
    }
    let tree_id = merge_index
        .write_tree_to(repository)
        .map_err(|error| map_remote_error(&error))?;
    let tree = repository
        .find_tree(tree_id)
        .map_err(|error| map_remote_error(&error))?;
    let message = format!("Merge remote-tracking branch '{remote_name}/{branch_name}'");
    let commit_id = repository
        .commit(
            None,
            &signature,
            &signature,
            &message,
            &tree,
            &[&local, &upstream],
        )
        .map_err(|error| map_remote_error(&error))?;
    let object = repository
        .find_object(commit_id, None)
        .map_err(|error| map_remote_error(&error))?;
    let mut checkout = CheckoutBuilder::new();
    checkout.safe();
    repository
        .checkout_tree(&object, Some(&mut checkout))
        .map_err(|_| {
            GitError::new(
                GitErrorKind::DirtyWorkingTree,
                "Pull could not update the working tree safely.",
                Some(
                    "No branch reference was changed. Refresh the repository and try again."
                        .to_owned(),
                ),
            )
        })?;
    repository
        .reference_matching(
            local_ref,
            commit_id,
            true,
            local_id,
            "Branchcraft pull: merge remote changes",
        )
        .map(|_| ())
        .map_err(|error| map_remote_error(&error))
}

fn update_tracking_reference(
    repository: &Repository,
    upstream_ref: &str,
    target: git2::Oid,
) -> Result<(), GitError> {
    repository
        .reference(
            upstream_ref,
            target,
            true,
            "Branchcraft push: update remote-tracking reference",
        )
        .map(|_| ())
        .map_err(|error| map_remote_error(&error))
}

fn operation_result(
    path: &Path,
    repository: &Repository,
    remote_name: &str,
    outcome: RemoteOutcome,
) -> Result<RemoteOperationResult, GitError> {
    Ok(RemoteOperationResult {
        repository: LibGitService::snapshot(path)?,
        remote: remote_info(repository, remote_name)?,
        outcome,
    })
}

fn missing_upstream(remote_name: &str, branch_name: &str) -> GitError {
    GitError::new(
        GitErrorKind::InvalidReference,
        "The remote branch could not be found.",
        Some(format!(
            "The branch {remote_name}/{branch_name} does not exist after fetching. Push this branch first or configure its upstream."
        )),
    )
}

fn detached_head(detail: &str) -> GitError {
    GitError::new(
        GitErrorKind::DetachedHead,
        "Remote changes require a local branch.",
        Some(detail.to_owned()),
    )
}

fn non_fast_forward() -> GitError {
    GitError::new(
        GitErrorKind::NonFastForward,
        "Push was rejected because the remote has newer commits.",
        Some("Pull the remote changes, review the result, and push again.".to_owned()),
    )
}

fn map_remote_error(error: &git2::Error) -> GitError {
    if error.code() == ErrorCode::Auth {
        GitError::new(
            GitErrorKind::Authentication,
            "GitHub rejected the stored credentials.",
            Some("Sign in with GitHub again, then retry the operation.".to_owned()),
        )
    } else if error.code() == ErrorCode::NotFastForward {
        non_fast_forward()
    } else if matches!(error.code(), ErrorCode::Conflict | ErrorCode::MergeConflict) {
        GitError::new(
            GitErrorKind::Conflict,
            "The remote operation encountered a merge conflict.",
            Some("Review the repository state before trying again.".to_owned()),
        )
    } else if matches!(
        error.class(),
        ErrorClass::Net | ErrorClass::Http | ErrorClass::Ssl | ErrorClass::Ssh
    ) || matches!(error.code(), ErrorCode::NotFound | ErrorCode::Timeout)
    {
        GitError::new(
            GitErrorKind::Network,
            "The remote repository could not be reached.",
            Some("Check the network connection and remote URL, then try again.".to_owned()),
        )
    } else {
        GitError::new(
            GitErrorKind::Unknown,
            "Branchcraft could not complete the remote operation.",
            Some("Refresh the repository and try again.".to_owned()),
        )
    }
}

fn emit(
    progress: &mut ProgressCallback<'_>,
    operation: RemoteOperation,
    phase: RemoteProgressPhase,
    remote_name: &str,
    current: u64,
    total: u64,
    bytes: u64,
) {
    progress(RemoteProgress {
        operation,
        phase,
        remote_name: remote_name.to_owned(),
        current,
        total,
        bytes,
    });
}

fn count(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

enum PushRejection {
    NonFastForward,
    Rejected,
}

#[cfg(test)]
mod tests {
    use super::*;
    use branchcraft_test_support::TestRepository;
    use git2::Repository;
    use std::fs;
    use tempfile::TempDir;

    struct RemoteFixture {
        _root: TempDir,
        source: TestRepository,
        client_path: std::path::PathBuf,
        bare_path: std::path::PathBuf,
    }

    impl RemoteFixture {
        fn new() -> Self {
            let source = TestRepository::with_initial_commit();
            let fixture_root = std::env::current_dir()
                .expect("current directory")
                .join("target")
                .join("branchcraft-remote-tests");
            fs::create_dir_all(&fixture_root).expect("remote fixture parent");
            let root = tempfile::Builder::new()
                .prefix("remote-")
                .tempdir_in(fixture_root)
                .expect("remote fixture root");
            let bare_path = root.path().join("remote.git");
            Repository::init_bare(&bare_path).expect("bare remote");
            let source_repository = Repository::open(source.path()).expect("source repository");
            source_repository
                .remote("origin", bare_path.to_string_lossy().as_ref())
                .expect("source remote");
            let mut remote = source_repository
                .find_remote("origin")
                .expect("source origin");
            remote
                .push(&["refs/heads/master:refs/heads/master"], None)
                .expect("seed remote");
            let client_path = root.path().join("client");
            Repository::clone(bare_path.to_string_lossy().as_ref(), &client_path)
                .expect("clone client");
            let client = Repository::open(&client_path).expect("client repository");
            let mut config = client.config().expect("client config");
            config
                .set_str("user.name", "Branchcraft Tests")
                .expect("name");
            config
                .set_str("user.email", "tests@branchcraft.local")
                .expect("email");
            Self {
                _root: root,
                source,
                client_path,
                bare_path,
            }
        }

        fn source_commit_and_push(&self, message: &str, contents: &str) {
            self.source_file_commit_and_push("README.md", message, contents);
        }

        fn source_file_commit_and_push(&self, path: &str, message: &str, contents: &str) {
            self.source.write(path, contents);
            self.source.stage(path);
            self.source.commit(message);
            let source = Repository::open(self.source.path()).expect("source repository");
            source
                .find_remote("origin")
                .expect("source origin")
                .push(&["refs/heads/master:refs/heads/master"], None)
                .expect("push source");
        }

        fn client_commit(&self, message: &str, contents: &str) -> git2::Oid {
            self.client_file_commit("README.md", message, contents)
        }

        fn client_file_commit(&self, path: &str, message: &str, contents: &str) -> git2::Oid {
            fs::write(self.client_path.join(path), contents).expect("write client change");
            let client = Repository::open(&self.client_path).expect("client repository");
            let mut index = client.index().expect("client index");
            index
                .add_path(Path::new(path))
                .expect("stage client change");
            index.write().expect("write client index");
            let signature = client.signature().expect("client signature");
            let tree_id = index.write_tree().expect("client tree id");
            let tree = client.find_tree(tree_id).expect("client tree");
            let parent = client
                .head()
                .expect("client head")
                .peel_to_commit()
                .expect("parent");
            client
                .commit(
                    Some("HEAD"),
                    &signature,
                    &signature,
                    message,
                    &tree,
                    &[&parent],
                )
                .expect("client commit")
        }
    }

    fn ignore_progress(_: RemoteProgress) {}

    #[test]
    fn reports_missing_and_unsupported_remotes_without_credentials() {
        let repository = TestRepository::with_initial_commit();
        assert!(
            primary_remote(repository.path())
                .expect("remote info")
                .is_none()
        );
        let error =
            fetch(repository.path(), None, &mut ignore_progress).expect_err("fetch without remote");
        assert_eq!(error.kind, GitErrorKind::NoRemote);

        let raw = Repository::open(repository.path()).expect("repository");
        raw.remote("origin", "git@github.com:owner/repository.git")
            .expect("SSH remote");
        let info = primary_remote(repository.path())
            .expect("remote info")
            .expect("configured remote");
        assert_eq!(info.kind, RemoteKind::Unsupported);
        let error =
            fetch(repository.path(), None, &mut ignore_progress).expect_err("unsupported fetch");
        assert_eq!(error.kind, GitErrorKind::UnsupportedRemote);
    }

    #[test]
    fn strips_credentials_from_remote_display_and_rejects_the_url() {
        let repository = TestRepository::with_initial_commit();
        let raw = Repository::open(repository.path()).expect("repository");
        raw.remote(
            "origin",
            "https://oauth-token@example.com/owner/repository.git",
        )
        .expect("credential remote");

        let info = primary_remote(repository.path())
            .expect("remote info")
            .expect("configured remote");

        assert_eq!(info.kind, RemoteKind::Unsupported);
        assert!(!info.display_url.contains("oauth-token"));
        let (kind, display) =
            classify_remote_url("https://github.com/owner/repository.git?token=value");
        assert_eq!(kind, RemoteKind::Unsupported);
        assert!(!display.contains("token=value"));
        assert_eq!(
            classify_remote_url("file://example.com/shared/repository.git").0,
            RemoteKind::Unsupported
        );
    }

    #[test]
    fn github_https_requires_backend_credentials_before_network_access() {
        let repository = TestRepository::with_initial_commit();
        let raw = Repository::open(repository.path()).expect("repository");
        raw.remote("origin", "https://github.com/example/branchcraft.git")
            .expect("GitHub HTTPS remote");

        let info = primary_remote(repository.path())
            .expect("remote info")
            .expect("configured remote");
        assert_eq!(info.kind, RemoteKind::GitHubHttps);

        let error = fetch(repository.path(), None, &mut ignore_progress)
            .expect_err("missing backend credential");
        assert_eq!(error.kind, GitErrorKind::Authentication);
    }

    #[test]
    fn invalid_local_remote_returns_a_safe_network_error() {
        let repository = TestRepository::with_initial_commit();
        let missing = repository.path().join("remote-does-not-exist.git");
        let raw = Repository::open(repository.path()).expect("repository");
        raw.remote("origin", missing.to_string_lossy().as_ref())
            .expect("invalid local remote");

        let error =
            fetch(repository.path(), None, &mut ignore_progress).expect_err("invalid remote");

        assert_eq!(error.kind, GitErrorKind::Network);
        assert!(
            !error
                .summary
                .contains(&missing.to_string_lossy().to_string())
        );
    }

    #[test]
    fn fetches_and_fast_forwards_from_a_local_remote() {
        let fixture = RemoteFixture::new();
        fixture.source_commit_and_push("Remote change", "remote change\n");
        let mut events = Vec::new();

        let fetched = fetch(&fixture.client_path, None, &mut |event| events.push(event))
            .expect("fetch remote");
        assert_eq!(fetched.outcome, RemoteOutcome::Fetched);
        assert_eq!(fetched.remote.behind, 1);
        assert!(events.iter().any(|event| {
            event.phase == RemoteProgressPhase::Completed
                && event.operation == RemoteOperation::Fetch
        }));

        let pulled = pull(&fixture.client_path, None, &mut ignore_progress).expect("pull remote");
        assert_eq!(pulled.outcome, RemoteOutcome::FastForwarded);
        assert_eq!(pulled.remote.behind, 0);
        assert_eq!(
            fs::read_to_string(fixture.client_path.join("README.md"))
                .expect("client readme")
                .trim_end(),
            "remote change"
        );
    }

    #[test]
    fn pushes_a_local_commit_and_updates_tracking_state() {
        let fixture = RemoteFixture::new();
        let commit_id = fixture.client_commit("Client change", "client change\n");

        let result = push(&fixture.client_path, None, &mut ignore_progress).expect("push remote");

        assert_eq!(result.outcome, RemoteOutcome::Pushed);
        assert_eq!(result.remote.ahead, 0);
        let bare = Repository::open_bare(&fixture.bare_path).expect("bare repository");
        assert_eq!(
            bare.find_reference("refs/heads/master")
                .expect("remote branch")
                .target(),
            Some(commit_id)
        );
    }

    #[test]
    fn pull_preserves_a_dirty_working_tree() {
        let fixture = RemoteFixture::new();
        fixture.source_commit_and_push("Remote change", "remote change\n");
        fs::write(fixture.client_path.join("README.md"), "local edit\n").expect("local edit");

        let error = pull(&fixture.client_path, None, &mut ignore_progress).expect_err("dirty pull");

        assert_eq!(error.kind, GitErrorKind::DirtyWorkingTree);
        assert_eq!(
            fs::read_to_string(fixture.client_path.join("README.md")).expect("client readme"),
            "local edit\n"
        );
    }

    #[test]
    fn pull_merges_non_conflicting_diverged_histories() {
        let fixture = RemoteFixture::new();
        fixture.client_file_commit("LOCAL.md", "Local addition", "local file\n");
        fixture.source_commit_and_push("Remote change", "remote version\n");

        let result =
            pull(&fixture.client_path, None, &mut ignore_progress).expect("non-conflicting pull");

        assert_eq!(result.outcome, RemoteOutcome::Merged);
        assert_eq!(result.remote.behind, 0);
        assert_eq!(result.repository.files, Vec::new());
        let client = Repository::open(&fixture.client_path).expect("client repository");
        assert_eq!(
            client
                .head()
                .expect("head")
                .peel_to_commit()
                .expect("merge commit")
                .parent_count(),
            2
        );
        assert_eq!(
            fs::read_to_string(fixture.client_path.join("README.md"))
                .expect("client readme")
                .trim_end(),
            "remote version"
        );
        assert!(fixture.client_path.join("LOCAL.md").is_file());
    }

    #[test]
    fn push_reports_a_non_fast_forward_rejection() {
        let fixture = RemoteFixture::new();
        fixture.client_file_commit("LOCAL.md", "Local addition", "local file\n");
        fixture.source_commit_and_push("Remote change", "remote version\n");

        let error = push(&fixture.client_path, None, &mut ignore_progress)
            .expect_err("non-fast-forward push");

        assert_eq!(error.kind, GitErrorKind::NonFastForward);
    }

    #[test]
    fn conflicting_pull_does_not_modify_head_or_working_files() {
        let fixture = RemoteFixture::new();
        let local_commit = fixture.client_commit("Local change", "local version\n");
        fixture.source_commit_and_push("Remote change", "remote version\n");

        let error =
            pull(&fixture.client_path, None, &mut ignore_progress).expect_err("conflicting pull");

        assert_eq!(error.kind, GitErrorKind::Conflict);
        let client = Repository::open(&fixture.client_path).expect("client repository");
        assert_eq!(client.head().expect("head").target(), Some(local_commit));
        assert_eq!(
            fs::read_to_string(fixture.client_path.join("README.md"))
                .expect("client readme")
                .trim_end(),
            "local version"
        );
        assert!(!client.index().expect("client index").has_conflicts());
    }
}
