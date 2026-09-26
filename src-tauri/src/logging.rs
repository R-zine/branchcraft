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
    fs::create_dir_all(directory)?;
    let path = directory.join("branchcraft.log");
    rotate_if_needed(&path)?;
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    let logger = LOGGER.get_or_init(|| BranchcraftLogger {
        file: Mutex::new(file),
    });
    log::set_logger(logger).map_err(|_| io::Error::other("logger already initialized"))?;
    log::set_max_level(LevelFilter::Info);
    Ok(path)
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
