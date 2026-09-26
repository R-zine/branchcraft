use super::{canonical_directory, changed_files, map_open_error};
use branchcraft_app_core::{
    ChangeKind, Diff, DiffFile, DiffFileKind, DiffHunk, DiffLine, DiffLineKind, DiffPreviewStatus,
    FileStatus, GitError, GitErrorKind,
};
use git2::{DiffOptions, ErrorCode, ObjectType, Oid, Patch, Repository};
use std::{
    fs::{self, File},
    io::{self, Read},
    path::Path,
};

/// Maximum bytes loaded from either side of a file diff.
pub const MAX_DIFF_FILE_BYTES: u64 = 2 * 1024 * 1024;

struct LoadedContent {
    bytes: Option<Vec<u8>>,
    size: u64,
    limitation: Option<DiffPreviewStatus>,
}

impl LoadedContent {
    const fn empty() -> Self {
        Self {
            bytes: Some(Vec::new()),
            size: 0,
            limitation: None,
        }
    }

    const fn limited(size: u64, limitation: DiffPreviewStatus) -> Self {
        Self {
            bytes: None,
            size,
            limitation: Some(limitation),
        }
    }
}

pub(super) fn file_diff(repository_path: &Path, file_path: &str) -> Result<Diff, GitError> {
    let selected_path = canonical_directory(repository_path)?;
    let repository = Repository::open(selected_path).map_err(|error| map_open_error(&error))?;
    let relative_path = super::validate_relative_path(file_path)?;
    let status = changed_files(&repository)?
        .into_iter()
        .find(|candidate| candidate.path == file_path)
        .ok_or_else(changed_file_missing)?;
    let old_display_path = status.previous_path.as_deref().unwrap_or(file_path);
    let old_relative_path = super::validate_relative_path(old_display_path)?;

    let old = load_head_content(&repository, &old_relative_path)?;
    let new = load_new_content(&repository, &relative_path, &status)?;
    let kind = diff_kind(&status);
    let preview_status = preview_status(&old, &new);
    let old_size = old.size;
    let new_size = new.size;

    if preview_status != DiffPreviewStatus::Ready {
        return Ok(Diff {
            file: DiffFile {
                path: status.path,
                previous_path: status.previous_path,
                kind,
                preview_status,
                old_content: None,
                new_content: None,
                old_size,
                new_size,
            },
            hunks: Vec::new(),
        });
    }

    let old_bytes = old.bytes.unwrap_or_default();
    let new_bytes = new.bytes.unwrap_or_default();
    let old_content = String::from_utf8(old_bytes).map_err(|_| invalid_text_error())?;
    let new_content = String::from_utf8(new_bytes).map_err(|_| invalid_text_error())?;
    let hunks = build_hunks(
        old_content.as_bytes(),
        &old_relative_path,
        new_content.as_bytes(),
        &relative_path,
    )?;

    Ok(Diff {
        file: DiffFile {
            path: status.path,
            previous_path: status.previous_path,
            kind,
            preview_status,
            old_content: Some(old_content),
            new_content: Some(new_content),
            old_size,
            new_size,
        },
        hunks,
    })
}

fn load_head_content(
    repository: &Repository,
    relative_path: &Path,
) -> Result<LoadedContent, GitError> {
    let head = match repository.head() {
        Ok(head) => head,
        Err(error) if error.code() == ErrorCode::UnbornBranch => {
            return Ok(LoadedContent::empty());
        }
        Err(error) => return Err(map_diff_git_error(&error)),
    };
    let tree = head
        .peel_to_tree()
        .map_err(|error| map_diff_git_error(&error))?;
    let entry = match tree.get_path(relative_path) {
        Ok(entry) => entry,
        Err(error) if error.code() == ErrorCode::NotFound => {
            return Ok(LoadedContent::empty());
        }
        Err(error) => return Err(map_diff_git_error(&error)),
    };

    if entry.kind() != Some(ObjectType::Blob) {
        return Ok(LoadedContent::limited(0, DiffPreviewStatus::Unsupported));
    }
    load_blob_content(repository, entry.id())
}

fn load_new_content(
    repository: &Repository,
    relative_path: &Path,
    status: &FileStatus,
) -> Result<LoadedContent, GitError> {
    let Some(workdir) = repository.workdir() else {
        return Ok(LoadedContent::limited(0, DiffPreviewStatus::Unsupported));
    };
    let root = fs::canonicalize(workdir).map_err(|error| map_diff_io_error(&error))?;
    let candidate = root.join(relative_path);
    let metadata = match fs::symlink_metadata(&candidate) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if status.unstaged == Some(ChangeKind::Deleted)
                || status.staged == Some(ChangeKind::Deleted)
            {
                return Ok(LoadedContent::empty());
            }
            return load_index_content(repository, relative_path);
        }
        Err(error) => return Err(map_diff_io_error(&error)),
    };

    if metadata.file_type().is_symlink() {
        let target = fs::read_link(candidate).map_err(|error| map_diff_io_error(&error))?;
        let bytes = target.to_string_lossy().as_bytes().to_vec();
        return Ok(content_from_bytes(bytes));
    }
    if !metadata.is_file() {
        return Ok(LoadedContent::limited(
            metadata.len(),
            DiffPreviewStatus::Unsupported,
        ));
    }

    let canonical_candidate =
        fs::canonicalize(&candidate).map_err(|error| map_diff_io_error(&error))?;
    if !canonical_candidate.starts_with(&root) {
        return Err(GitError::new(
            GitErrorKind::InvalidPath,
            "The selected file resolves outside the repository.",
            None,
        ));
    }
    if metadata.len() > MAX_DIFF_FILE_BYTES {
        return Ok(LoadedContent::limited(
            metadata.len(),
            DiffPreviewStatus::TooLarge,
        ));
    }

    let mut bytes = Vec::with_capacity(usize::try_from(metadata.len()).unwrap_or_default());
    File::open(canonical_candidate)
        .map_err(|error| map_diff_io_error(&error))?
        .take(MAX_DIFF_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| map_diff_io_error(&error))?;
    if u64::try_from(bytes.len()).unwrap_or(u64::MAX) > MAX_DIFF_FILE_BYTES {
        return Ok(LoadedContent::limited(
            u64::try_from(bytes.len()).unwrap_or(u64::MAX),
            DiffPreviewStatus::TooLarge,
        ));
    }
    Ok(content_from_bytes(bytes))
}

fn load_index_content(
    repository: &Repository,
    relative_path: &Path,
) -> Result<LoadedContent, GitError> {
    let index = repository
        .index()
        .map_err(|error| map_diff_git_error(&error))?;
    let Some(entry) = index.get_path(relative_path, 0) else {
        return Ok(LoadedContent::empty());
    };
    load_blob_content(repository, entry.id)
}

fn load_blob_content(repository: &Repository, oid: Oid) -> Result<LoadedContent, GitError> {
    let odb = repository
        .odb()
        .map_err(|error| map_diff_git_error(&error))?;
    let (size, object_type) = odb
        .read_header(oid)
        .map_err(|error| map_diff_git_error(&error))?;
    let size = u64::try_from(size).unwrap_or(u64::MAX);
    if object_type != ObjectType::Blob {
        return Ok(LoadedContent::limited(size, DiffPreviewStatus::Unsupported));
    }
    if size > MAX_DIFF_FILE_BYTES {
        return Ok(LoadedContent::limited(size, DiffPreviewStatus::TooLarge));
    }
    let blob = repository
        .find_blob(oid)
        .map_err(|error| map_diff_git_error(&error))?;
    Ok(content_from_bytes(blob.content().to_vec()))
}

fn content_from_bytes(bytes: Vec<u8>) -> LoadedContent {
    let size = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    LoadedContent {
        bytes: Some(bytes),
        size,
        limitation: None,
    }
}

fn preview_status(old: &LoadedContent, new: &LoadedContent) -> DiffPreviewStatus {
    if old.limitation == Some(DiffPreviewStatus::TooLarge)
        || new.limitation == Some(DiffPreviewStatus::TooLarge)
    {
        return DiffPreviewStatus::TooLarge;
    }
    if old.limitation == Some(DiffPreviewStatus::Unsupported)
        || new.limitation == Some(DiffPreviewStatus::Unsupported)
    {
        return DiffPreviewStatus::Unsupported;
    }

    let old_bytes = old.bytes.as_deref().unwrap_or_default();
    let new_bytes = new.bytes.as_deref().unwrap_or_default();
    if is_binary(old_bytes) || is_binary(new_bytes) {
        DiffPreviewStatus::Binary
    } else {
        DiffPreviewStatus::Ready
    }
}

fn is_binary(bytes: &[u8]) -> bool {
    bytes.contains(&0) || std::str::from_utf8(bytes).is_err()
}

fn diff_kind(status: &FileStatus) -> DiffFileKind {
    if status.conflicted {
        DiffFileKind::Conflicted
    } else if status.previous_path.is_some() {
        DiffFileKind::Renamed
    } else if status.staged == Some(ChangeKind::TypeChanged)
        || status.unstaged == Some(ChangeKind::TypeChanged)
    {
        DiffFileKind::TypeChanged
    } else if status.staged == Some(ChangeKind::Deleted)
        || status.unstaged == Some(ChangeKind::Deleted)
    {
        DiffFileKind::Deleted
    } else if matches!(
        status.staged,
        Some(ChangeKind::Added | ChangeKind::Untracked)
    ) || matches!(
        status.unstaged,
        Some(ChangeKind::Added | ChangeKind::Untracked)
    ) {
        DiffFileKind::Added
    } else {
        DiffFileKind::Modified
    }
}

fn build_hunks(
    old: &[u8],
    old_path: &Path,
    new: &[u8],
    new_path: &Path,
) -> Result<Vec<DiffHunk>, GitError> {
    let mut options = DiffOptions::new();
    options.context_lines(3).interhunk_lines(0).minimal(true);
    let patch = Patch::from_buffers(old, Some(old_path), new, Some(new_path), Some(&mut options))
        .map_err(|error| map_diff_git_error(&error))?;
    let mut hunks = Vec::with_capacity(patch.num_hunks());

    for hunk_index in 0..patch.num_hunks() {
        let (hunk, line_count) = patch
            .hunk(hunk_index)
            .map_err(|error| map_diff_git_error(&error))?;
        let mut lines = Vec::with_capacity(line_count);
        for line_index in 0..line_count {
            let line = patch
                .line_in_hunk(hunk_index, line_index)
                .map_err(|error| map_diff_git_error(&error))?;
            let kind = match line.origin() {
                '+' => DiffLineKind::Added,
                '-' => DiffLineKind::Deleted,
                '=' | '>' | '<' => DiffLineKind::NoNewline,
                _ => DiffLineKind::Context,
            };
            lines.push(DiffLine {
                kind,
                old_line_number: line.old_lineno(),
                new_line_number: line.new_lineno(),
                content: String::from_utf8_lossy(line.content()).into_owned(),
            });
        }
        hunks.push(DiffHunk {
            header: String::from_utf8_lossy(hunk.header())
                .trim_end_matches(['\r', '\n'])
                .to_owned(),
            old_start: hunk.old_start(),
            old_lines: hunk.old_lines(),
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
            lines,
        });
    }
    Ok(hunks)
}

fn changed_file_missing() -> GitError {
    GitError::new(
        GitErrorKind::InvalidPath,
        "This file is no longer changed.",
        Some("Refresh the repository to update the changes list.".to_owned()),
    )
}

fn invalid_text_error() -> GitError {
    GitError::new(
        GitErrorKind::Unknown,
        "Branchcraft could not decode this file.",
        Some("The file is not valid UTF-8 and cannot be rendered as source text.".to_owned()),
    )
}

fn map_diff_io_error(error: &io::Error) -> GitError {
    if error.kind() == io::ErrorKind::PermissionDenied {
        GitError::new(
            GitErrorKind::PermissionDenied,
            "Branchcraft does not have permission to read this file.",
            None,
        )
    } else {
        GitError::new(
            GitErrorKind::Unknown,
            "Branchcraft could not read this file.",
            Some("The file may have moved while the repository was being inspected.".to_owned()),
        )
    }
}

fn map_diff_git_error(error: &git2::Error) -> GitError {
    if error.code() == ErrorCode::Auth {
        GitError::new(
            GitErrorKind::PermissionDenied,
            "Branchcraft could not read this file.",
            None,
        )
    } else {
        GitError::new(
            GitErrorKind::Unknown,
            "Branchcraft could not compare this file.",
            Some("The repository may have changed during the diff operation.".to_owned()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::MAX_DIFF_FILE_BYTES;
    use crate::{GitService, LibGitService};
    use branchcraft_app_core::{DiffFileKind, DiffLineKind, DiffPreviewStatus};
    use branchcraft_test_support::TestRepository;

    #[test]
    fn diffs_modified_text_with_structured_hunks() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "# Fixture\n\nchanged\n");

        let diff = LibGitService::new()
            .diff(repository.path(), "README.md")
            .expect("text diff");

        assert_eq!(diff.file.kind, DiffFileKind::Modified);
        assert_eq!(diff.file.preview_status, DiffPreviewStatus::Ready);
        assert_eq!(diff.file.old_content.as_deref(), Some("# Fixture\n"));
        assert_eq!(
            diff.file.new_content.as_deref(),
            Some("# Fixture\n\nchanged\n")
        );
        assert!(
            diff.hunks.iter().flat_map(|hunk| &hunk.lines).any(|line| {
                line.kind == DiffLineKind::Added && line.content.contains("changed")
            })
        );
    }

    #[test]
    fn supports_added_deleted_renamed_and_empty_files() {
        let added = TestRepository::with_initial_commit();
        added.write("empty.txt", "");
        let added_diff = LibGitService::new()
            .diff(added.path(), "empty.txt")
            .expect("added empty file diff");
        assert_eq!(added_diff.file.kind, DiffFileKind::Added);
        assert_eq!(added_diff.file.old_content.as_deref(), Some(""));
        assert_eq!(added_diff.file.new_content.as_deref(), Some(""));

        let deleted = TestRepository::with_initial_commit();
        deleted.remove("delete-me.txt");
        let deleted_diff = LibGitService::new()
            .diff(deleted.path(), "delete-me.txt")
            .expect("deleted file diff");
        assert_eq!(deleted_diff.file.kind, DiffFileKind::Deleted);
        assert_eq!(deleted_diff.file.new_content.as_deref(), Some(""));

        let renamed = TestRepository::with_initial_commit();
        renamed.rename_and_stage("delete-me.txt", "archive/kept.txt");
        let renamed_diff = LibGitService::new()
            .diff(renamed.path(), "archive/kept.txt")
            .expect("renamed file diff");
        assert_eq!(renamed_diff.file.kind, DiffFileKind::Renamed);
        assert_eq!(
            renamed_diff.file.previous_path.as_deref(),
            Some("delete-me.txt")
        );
        assert_eq!(renamed_diff.file.old_content, renamed_diff.file.new_content);
    }

    #[test]
    fn detects_binary_files_without_sending_content() {
        let repository = TestRepository::with_initial_commit();
        repository.write_bytes("image.bin", &[0, 1, 2, 3]);

        let diff = LibGitService::new()
            .diff(repository.path(), "image.bin")
            .expect("binary diff");

        assert_eq!(diff.file.preview_status, DiffPreviewStatus::Binary);
        assert!(diff.file.old_content.is_none());
        assert!(diff.file.new_content.is_none());
        assert!(diff.hunks.is_empty());
    }

    #[test]
    fn refuses_oversized_files_before_returning_content() {
        let repository = TestRepository::with_initial_commit();
        let contents = vec![b'x'; usize::try_from(MAX_DIFF_FILE_BYTES + 1).unwrap()];
        repository.write_bytes("large.txt", &contents);

        let diff = LibGitService::new()
            .diff(repository.path(), "large.txt")
            .expect("large diff fallback");

        assert_eq!(diff.file.preview_status, DiffPreviewStatus::TooLarge);
        assert_eq!(diff.file.new_size, MAX_DIFF_FILE_BYTES + 1);
        assert!(diff.file.new_content.is_none());
    }

    #[test]
    fn preserves_whitespace_only_changes() {
        let repository = TestRepository::with_initial_commit();
        repository.write("README.md", "# Fixture  \n");

        let diff = LibGitService::new()
            .diff(repository.path(), "README.md")
            .expect("whitespace diff");

        assert_eq!(diff.file.new_content.as_deref(), Some("# Fixture  \n"));
        assert!(!diff.hunks.is_empty());
    }

    #[test]
    fn rejects_paths_outside_the_repository() {
        let repository = TestRepository::with_initial_commit();
        let error = LibGitService::new()
            .diff(repository.path(), "../secret.txt")
            .expect_err("path traversal must fail");

        assert_eq!(error.kind, branchcraft_app_core::GitErrorKind::InvalidPath);
    }
}
