//! Git hook discovery and bounded output streaming.

use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::mpsc,
    thread,
};
use thiserror::Error;

#[cfg(windows)]
use std::{ffi::OsString, fs, io};

const OUTPUT_CHUNK_BYTES: usize = 4 * 1024;
const MAX_OUTPUT_BYTES: usize = 1024 * 1024;

/// Commit-related hooks supported by Branchcraft's libgit2 commit path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum GitHook {
    PreCommit,
    PrepareCommitMsg,
    CommitMsg,
    PostCommit,
}

impl GitHook {
    #[must_use]
    pub const fn file_name(self) -> &'static str {
        match self {
            Self::PreCommit => "pre-commit",
            Self::PrepareCommitMsg => "prepare-commit-msg",
            Self::CommitMsg => "commit-msg",
            Self::PostCommit => "post-commit",
        }
    }

    #[must_use]
    pub const fn display_name(self) -> &'static str {
        self.file_name()
    }
}

/// Lifecycle or output category streamed while one hook executes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HookEventKind {
    Started,
    Stdout,
    Stderr,
    Succeeded,
    Failed,
}

/// A bounded terminal update suitable for forwarding over Tauri IPC.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    pub hook: GitHook,
    pub kind: HookEventKind,
    pub text: Option<String>,
    pub exit_code: Option<i32>,
}

impl HookEvent {
    fn lifecycle(hook: GitHook, kind: HookEventKind, exit_code: Option<i32>) -> Self {
        Self {
            hook,
            kind,
            text: None,
            exit_code,
        }
    }

    fn output(hook: GitHook, kind: HookEventKind, text: String) -> Self {
        Self {
            hook,
            kind,
            text: Some(text),
            exit_code: None,
        }
    }
}

/// Why a hook could not complete successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookErrorKind {
    Start,
    Output,
    Rejected,
}

/// Failure returned to the Git operation after the terminal has been updated.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("{hook:?} hook failed ({kind:?})")]
pub struct HookError {
    pub hook: GitHook,
    pub kind: HookErrorKind,
    pub exit_code: Option<i32>,
}

/// Whether a hook was absent or completed successfully.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookOutcome {
    NotPresent,
    Passed,
}

/// Resolves repository hooks and runs them in the repository worktree.
#[derive(Debug, Clone)]
pub struct HookRunner {
    worktree: PathBuf,
    git_dir: PathBuf,
    hooks_dir: PathBuf,
}

impl HookRunner {
    /// Builds a runner for a non-bare repository. A configured relative
    /// `core.hooksPath` is resolved from the worktree, matching Git's behavior.
    #[must_use]
    pub fn new(worktree: &Path, git_dir: &Path, configured_hooks_path: Option<&Path>) -> Self {
        let hooks_dir = configured_hooks_path.map_or_else(
            || git_dir.join("hooks"),
            |path| {
                if path.is_absolute() {
                    path.to_path_buf()
                } else {
                    worktree.join(path)
                }
            },
        );
        Self {
            worktree: worktree.to_path_buf(),
            git_dir: git_dir.to_path_buf(),
            hooks_dir,
        }
    }

    /// Runs one hook and streams bounded stdout/stderr updates.
    ///
    /// # Errors
    ///
    /// Returns an error when the hook cannot start, its output cannot be read,
    /// or it exits unsuccessfully.
    pub fn run(
        &self,
        hook: GitHook,
        arguments: &[&Path],
        on_event: &mut dyn FnMut(HookEvent),
    ) -> Result<HookOutcome, HookError> {
        let hook_path = self.hooks_dir.join(hook.file_name());
        if !is_runnable_hook(&hook_path) {
            return Ok(HookOutcome::NotPresent);
        }

        on_event(HookEvent::lifecycle(hook, HookEventKind::Started, None));
        #[cfg(unix)]
        let mut command = command_for_hook(&hook_path);
        #[cfg(windows)]
        let mut command = command_for_hook(&hook_path).map_err(|_| {
            on_event(HookEvent::output(
                hook,
                HookEventKind::Stderr,
                "Branchcraft could not determine how to start this hook.\n".to_owned(),
            ));
            on_event(HookEvent::lifecycle(hook, HookEventKind::Failed, None));
            HookError {
                hook,
                kind: HookErrorKind::Start,
                exit_code: None,
            }
        })?;
        command
            .args(arguments)
            .current_dir(&self.worktree)
            .env("GIT_DIR", &self.git_dir)
            .env("GIT_WORK_TREE", &self.worktree)
            .env("GIT_EDITOR", ":")
            .env("BRANCHCRAFT", "1")
            .env_remove("GIT_INDEX_FILE")
            .env_remove("GIT_OBJECT_DIRECTORY")
            .env_remove("GIT_ALTERNATE_OBJECT_DIRECTORIES")
            .env_remove("GIT_COMMON_DIR")
            .env_remove("GIT_PREFIX")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|_| {
            on_event(HookEvent::output(
                hook,
                HookEventKind::Stderr,
                "Branchcraft could not start this hook. Check its interpreter and permissions.\n"
                    .to_owned(),
            ));
            on_event(HookEvent::lifecycle(hook, HookEventKind::Failed, None));
            HookError {
                hook,
                kind: HookErrorKind::Start,
                exit_code: None,
            }
        })?;

        let status = stream_output(&mut child, hook, on_event)?;
        if status.success() {
            on_event(HookEvent::lifecycle(
                hook,
                HookEventKind::Succeeded,
                status.code(),
            ));
            Ok(HookOutcome::Passed)
        } else {
            on_event(HookEvent::lifecycle(
                hook,
                HookEventKind::Failed,
                status.code(),
            ));
            Err(HookError {
                hook,
                kind: HookErrorKind::Rejected,
                exit_code: status.code(),
            })
        }
    }
}

enum OutputUpdate {
    Chunk(HookEventKind, Vec<u8>),
    ReadFailed,
}

fn stream_output(
    child: &mut Child,
    hook: GitHook,
    on_event: &mut dyn FnMut(HookEvent),
) -> Result<ExitStatus, HookError> {
    let stdout = child.stdout.take().ok_or(HookError {
        hook,
        kind: HookErrorKind::Output,
        exit_code: None,
    })?;
    let stderr = child.stderr.take().ok_or(HookError {
        hook,
        kind: HookErrorKind::Output,
        exit_code: None,
    })?;
    let (sender, receiver) = mpsc::channel();
    let stdout_reader = output_reader(stdout, HookEventKind::Stdout, sender.clone());
    let stderr_reader = output_reader(stderr, HookEventKind::Stderr, sender);
    let mut emitted_bytes = 0_usize;
    let mut truncated = false;
    let mut output_failed = false;

    for update in receiver {
        match update {
            OutputUpdate::Chunk(kind, bytes) => {
                emit_output_chunk(
                    hook,
                    kind,
                    &bytes,
                    &mut emitted_bytes,
                    &mut truncated,
                    on_event,
                );
            }
            OutputUpdate::ReadFailed => output_failed = true,
        }
    }

    let stdout_joined = stdout_reader.join().is_ok();
    let stderr_joined = stderr_reader.join().is_ok();
    let status = child.wait().map_err(|_| HookError {
        hook,
        kind: HookErrorKind::Output,
        exit_code: None,
    })?;
    if output_failed || !stdout_joined || !stderr_joined {
        on_event(HookEvent::output(
            hook,
            HookEventKind::Stderr,
            "Branchcraft could not read all hook output.\n".to_owned(),
        ));
        on_event(HookEvent::lifecycle(
            hook,
            HookEventKind::Failed,
            status.code(),
        ));
        return Err(HookError {
            hook,
            kind: HookErrorKind::Output,
            exit_code: status.code(),
        });
    }
    Ok(status)
}

fn emit_output_chunk(
    hook: GitHook,
    kind: HookEventKind,
    bytes: &[u8],
    emitted_bytes: &mut usize,
    truncated: &mut bool,
    on_event: &mut dyn FnMut(HookEvent),
) {
    if *emitted_bytes < MAX_OUTPUT_BYTES {
        let remaining = MAX_OUTPUT_BYTES - *emitted_bytes;
        let visible = &bytes[..bytes.len().min(remaining)];
        *emitted_bytes += visible.len();
        if !visible.is_empty() {
            on_event(HookEvent::output(
                hook,
                kind,
                String::from_utf8_lossy(visible).into_owned(),
            ));
        }
    }
    if *emitted_bytes == MAX_OUTPUT_BYTES && !*truncated {
        *truncated = true;
        on_event(HookEvent::output(
            hook,
            HookEventKind::Stderr,
            "\n[Branchcraft stopped displaying output after 1 MiB.]\n".to_owned(),
        ));
    }
}

fn output_reader(
    mut reader: impl Read + Send + 'static,
    kind: HookEventKind,
    sender: mpsc::Sender<OutputUpdate>,
) -> thread::JoinHandle<()> {
    thread::spawn(move || {
        let mut buffer = vec![0_u8; OUTPUT_CHUNK_BYTES];
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => break,
                Ok(length) => {
                    if sender
                        .send(OutputUpdate::Chunk(kind, buffer[..length].to_vec()))
                        .is_err()
                    {
                        break;
                    }
                }
                Err(_) => {
                    let _ = sender.send(OutputUpdate::ReadFailed);
                    break;
                }
            }
        }
    })
}

#[cfg(unix)]
fn is_runnable_hook(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.metadata()
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
fn is_runnable_hook(path: &Path) -> bool {
    path.is_file()
}

#[cfg(unix)]
fn command_for_hook(path: &Path) -> Command {
    Command::new(path)
}

#[cfg(windows)]
fn command_for_hook(path: &Path) -> io::Result<Command> {
    let contents = fs::read(path)?;
    if contents.starts_with(b"MZ") {
        return Ok(Command::new(path));
    }
    let first_line = contents
        .split(|byte| *byte == b'\n')
        .next()
        .map(|line| String::from_utf8_lossy(line).trim().to_owned())
        .unwrap_or_default();
    let declaration = first_line.strip_prefix("#!").ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "Windows hooks need a shebang or native executable format",
        )
    })?;
    let mut parts = declaration.split_whitespace();
    let declared_program = parts
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "hook shebang is empty"))?;
    let declared_name = Path::new(declared_program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(declared_program);
    let (program, arguments) = if declared_name.eq_ignore_ascii_case("env")
        || declared_name.eq_ignore_ascii_case("env.exe")
    {
        let mut remaining = parts.collect::<Vec<_>>();
        if remaining.first().is_some_and(|part| *part == "-S") {
            remaining.remove(0);
        }
        let interpreter = remaining.first().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, "env shebang has no interpreter")
        })?;
        let program = resolve_windows_interpreter(interpreter);
        (program, remaining.into_iter().skip(1).collect::<Vec<_>>())
    } else {
        (
            resolve_windows_interpreter(declared_name),
            parts.collect::<Vec<_>>(),
        )
    };
    let mut command = Command::new(program);
    command.args(arguments).arg(path);
    Ok(command)
}

#[cfg(windows)]
fn resolve_windows_interpreter(name: &str) -> OsString {
    let normalized = Path::new(name)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    if normalized.eq_ignore_ascii_case("sh") || normalized.eq_ignore_ascii_case("bash") {
        let executable = format!("{normalized}.exe");
        let mut candidates = Vec::new();
        if let Some(program_files) = std::env::var_os("ProgramFiles") {
            candidates.push(
                PathBuf::from(program_files)
                    .join("Git")
                    .join("bin")
                    .join(&executable),
            );
        }
        if let Some(local_app_data) = std::env::var_os("LOCALAPPDATA") {
            candidates.push(
                PathBuf::from(local_app_data)
                    .join("Programs")
                    .join("Git")
                    .join("bin")
                    .join(&executable),
            );
        }
        if let Some(candidate) = candidates.into_iter().find(|candidate| candidate.is_file()) {
            return candidate.into_os_string();
        }
    }
    OsString::from(normalized)
}

#[cfg(test)]
mod tests {
    use super::{GitHook, HookEvent, HookEventKind, HookOutcome, HookRunner};
    use std::{fs, path::Path};
    use tempfile::TempDir;

    fn fixture() -> (TempDir, HookRunner) {
        let root = TempDir::new().expect("hook fixture");
        let git_dir = root.path().join(".git");
        fs::create_dir_all(git_dir.join("hooks")).expect("hook directory");
        let runner = HookRunner::new(root.path(), &git_dir, None);
        (root, runner)
    }

    #[cfg(unix)]
    fn write_hook(path: &Path, body: &str) {
        use std::os::unix::fs::PermissionsExt;

        fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write hook");
        let mut permissions = path.metadata().expect("hook metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions).expect("hook permissions");
    }

    #[cfg(windows)]
    fn write_hook(path: &Path, body: &str) {
        fs::write(path, format!("#!/bin/sh\n{body}\n")).expect("write hook");
    }

    #[test]
    fn skips_a_missing_hook_without_opening_the_terminal() {
        let (_root, runner) = fixture();
        let mut events = Vec::new();
        let outcome = runner
            .run(GitHook::PreCommit, &[], &mut |event| events.push(event))
            .expect("missing hook is optional");

        assert_eq!(outcome, HookOutcome::NotPresent);
        assert_eq!(events, Vec::<HookEvent>::new());
    }

    #[test]
    fn streams_stdout_stderr_and_success() {
        let (root, runner) = fixture();
        let path = root.path().join(".git").join("hooks").join("pre-commit");
        write_hook(
            &path,
            "printf 'checking stdout\\n'\nprintf 'checking stderr\\n' >&2",
        );
        let mut events = Vec::<HookEvent>::new();

        let outcome = runner
            .run(GitHook::PreCommit, &[], &mut |event| events.push(event))
            .expect("successful hook");

        assert_eq!(outcome, HookOutcome::Passed);
        assert_eq!(
            events.first().map(|event| event.kind),
            Some(HookEventKind::Started)
        );
        assert_eq!(
            events.last().map(|event| event.kind),
            Some(HookEventKind::Succeeded)
        );
        let output = events
            .iter()
            .filter_map(|event| event.text.as_deref())
            .collect::<String>();
        assert!(output.contains("checking stdout"));
        assert!(output.contains("checking stderr"));
    }

    #[test]
    fn reports_a_rejected_hook_with_its_exit_code() {
        let (root, runner) = fixture();
        let path = root.path().join(".git").join("hooks").join("commit-msg");
        write_hook(&path, "printf 'rejected\\n' >&2\nexit 7");

        let error = runner
            .run(GitHook::CommitMsg, &[Path::new("message")], &mut |_| {})
            .expect_err("hook rejection");

        assert_eq!(error.exit_code, Some(7));
    }

    #[test]
    fn resolves_a_relative_custom_hooks_directory_from_the_worktree() {
        let root = TempDir::new().expect("hook fixture");
        let git_dir = root.path().join(".git");
        let custom = root.path().join("tools").join("hooks");
        fs::create_dir_all(&custom).expect("custom hooks");
        let path = custom.join("post-commit");
        write_hook(&path, "printf 'custom\\n'");
        let runner = HookRunner::new(root.path(), &git_dir, Some(Path::new("tools/hooks")));

        assert_eq!(
            runner.run(GitHook::PostCommit, &[], &mut |_| {}),
            Ok(HookOutcome::Passed)
        );
    }
}
