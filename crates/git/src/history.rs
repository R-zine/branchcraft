use super::{canonical_directory, map_open_error};
use branchcraft_app_core::{CommitHistory, GitError, GitErrorKind, HistoryCommit};
use git2::{ErrorCode, Repository, Sort};
use std::path::Path;

const HISTORY_LIMIT: usize = 200;

pub(super) fn history(repository_path: &Path) -> Result<CommitHistory, GitError> {
    let selected_path = canonical_directory(repository_path)?;
    let repository = Repository::open(selected_path).map_err(|error| map_open_error(&error))?;
    if repository
        .head()
        .is_err_and(|error| matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound))
    {
        return Ok(CommitHistory {
            commits: Vec::new(),
            truncated: false,
        });
    }
    let mut walk = repository.revwalk().map_err(|error| map_error(&error))?;
    if let Err(error) = walk.push_head() {
        if matches!(error.code(), ErrorCode::UnbornBranch | ErrorCode::NotFound) {
            return Ok(CommitHistory {
                commits: Vec::new(),
                truncated: false,
            });
        }
        return Err(map_error(&error));
    }
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)
        .map_err(|error| map_error(&error))?;

    let mut commits = Vec::with_capacity(HISTORY_LIMIT);
    let mut truncated = false;
    for oid in walk {
        if commits.len() == HISTORY_LIMIT {
            truncated = true;
            break;
        }
        let oid = oid.map_err(|error| map_error(&error))?;
        let commit = repository
            .find_commit(oid)
            .map_err(|error| map_error(&error))?;
        let id = oid.to_string();
        let author = commit.author();
        let authored_at = author.when();
        let description = String::from_utf8_lossy(commit.body_bytes().unwrap_or_default())
            .trim()
            .to_owned();

        commits.push(HistoryCommit {
            short_id: id.chars().take(8).collect(),
            id,
            summary: String::from_utf8_lossy(
                commit.summary_bytes().unwrap_or(b"(no commit message)"),
            )
            .into_owned(),
            description: (!description.is_empty()).then_some(description),
            author_name: String::from_utf8_lossy(author.name_bytes()).into_owned(),
            author_email: String::from_utf8_lossy(author.email_bytes()).into_owned(),
            authored_at: authored_at.seconds(),
            author_offset_minutes: authored_at.offset_minutes(),
            parent_ids: commit
                .parent_ids()
                .map(|parent| parent.to_string())
                .collect(),
        });
    }

    Ok(CommitHistory { commits, truncated })
}

fn map_error(error: &git2::Error) -> GitError {
    GitError::new(
        GitErrorKind::Unknown,
        "Branchcraft could not read the commit history.",
        Some(if error.code() == ErrorCode::NotFound {
            "A commit disappeared while the history was being read.".to_owned()
        } else {
            "The repository may be damaged or changing concurrently.".to_owned()
        }),
    )
}

#[cfg(test)]
mod tests {
    use crate::{GitService, LibGitService};
    use branchcraft_test_support::TestRepository;

    #[test]
    fn returns_newest_first_with_commit_metadata() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "second\n");
        repository.stage("README.md");
        repository.commit("Second commit\n\nMore detail");

        let history = LibGitService::new()
            .history(repository.path())
            .expect("read history");

        assert_eq!(history.commits.len(), 2);
        assert_eq!(history.commits[0].summary, "Second commit");
        assert_eq!(
            history.commits[0].description.as_deref(),
            Some("More detail")
        );
        assert_eq!(history.commits[0].author_name, "Branchcraft Tests");
        assert_eq!(history.commits[0].short_id.len(), 8);
        assert_eq!(history.commits[0].parent_ids.len(), 1);
        assert!(!history.truncated);
    }

    #[test]
    fn empty_and_detached_repositories_have_stable_history() {
        let empty = TestRepository::empty();
        assert_eq!(
            LibGitService::new()
                .history(empty.path())
                .expect("empty history")
                .commits,
            Vec::new()
        );

        let detached = TestRepository::with_initial_commit();
        detached.detach_head();
        assert_eq!(
            LibGitService::new()
                .history(detached.path())
                .expect("detached history")
                .commits
                .len(),
            1
        );
    }
}
