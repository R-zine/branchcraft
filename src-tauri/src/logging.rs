use log::{LevelFilter, Log, Metadata, Record};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

const MAX_LOG_BYTES: u64 = 1024 * 1024;
static LOGGER: OnceLock<BranchcraftLogger> = OnceLock::new();

struct BranchcraftLogger {
    file: Mutex<File>,
}

impl Log for BranchcraftLogger {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        metadata.level() <= log::Level::Info && metadata.target().starts_with("branchcraft")
    }

    fn log(&self, record: &Record<'_>) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let seconds = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |duration| duration.as_secs());
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(
                file,
                "{seconds} {} {} - {}",
                record.level(),
                record.target(),
                record.args()
            );
        }
    }

    fn flush(&self) {
        if let Ok(mut file) = self.file.lock() {
            let _ = file.flush();
        }
    }
}

pub fn initialize(directory: &Path) -> io::Result<PathBuf> {
    let (path, file) = prepare_log_file(directory)?;
    let logger = LOGGER.get_or_init(|| BranchcraftLogger {
        file: Mutex::new(file),
    });
    log::set_logger(logger).map_err(|_| io::Error::other("logger already initialized"))?;
    log::set_max_level(LevelFilter::Info);
    Ok(path)
}

fn prepare_log_file(directory: &Path) -> io::Result<(PathBuf, File)> {
    fs::create_dir_all(directory)?;
    let path = directory.join("branchcraft.log");
    rotate_if_needed(&path)?;
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    Ok((path, file))
}

fn rotate_if_needed(path: &Path) -> io::Result<()> {
    let Ok(metadata) = fs::metadata(path) else {
        return Ok(());
    };
    if metadata.len() <= MAX_LOG_BYTES {
        return Ok(());
    }
    let previous = path.with_extension("previous.log");
    if previous.exists() {
        fs::remove_file(&previous)?;
    }
    fs::rename(path, previous)
}

#[cfg(test)]
mod tests {
    use super::{MAX_LOG_BYTES, prepare_log_file};
    use std::{fs, io::Write};

    #[test]
    fn creates_and_rotates_windows_safe_log_files() {
        let root = tempfile::tempdir().expect("log root");
        let directory = root.path().join("Branchcraft Logs");
        fs::create_dir_all(&directory).expect("log directory");
        let current = directory.join("branchcraft.log");
        fs::write(
            &current,
            vec![b'x'; usize::try_from(MAX_LOG_BYTES + 1).expect("test log size")],
        )
        .expect("oversized log");

        let (path, mut file) = prepare_log_file(&directory).expect("prepare log file");
        file.write_all(b"new log entry\n").expect("write new log");
        file.sync_all().expect("sync new log");

        assert_eq!(path, current);
        assert_eq!(
            fs::read_to_string(&path).expect("new log contents"),
            "new log entry\n"
        );
        assert_eq!(
            fs::metadata(directory.join("branchcraft.previous.log"))
                .expect("rotated log")
                .len(),
            MAX_LOG_BYTES + 1
        );
    }
}
