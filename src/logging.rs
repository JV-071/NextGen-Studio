use log::{LevelFilter, Log, Metadata, Record};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
struct Logger {
    file: Mutex<File>,
    path: PathBuf,
}
impl Log for Logger {
    fn enabled(&self, m: &Metadata) -> bool {
        m.level() <= log::Level::Info
    }
    fn log(&self, r: &Record) {
        if !self.enabled(r.metadata()) {
            return;
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut line = format!("{now} [{}] {}: {}\n", r.level(), r.target(), r.args());
        if line.len() > 65536 {
            let mut end = 65536;
            while !line.is_char_boundary(end) {
                end -= 1;
            }
            line.truncate(end);
            line.push_str(" [truncated]\n");
        }
        if cfg!(debug_assertions) {
            eprint!("{line}");
        }
        if let Ok(mut f) = self.file.lock() {
            if f.metadata()
                .is_ok_and(|m| m.len() + line.len() as u64 > 2 * 1024 * 1024)
            {
                let old = self.path.with_extension("log.1");
                let _ = fs::remove_file(&old);
                if fs::rename(&self.path, &old).is_ok() {
                    if let Ok(next) = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&self.path)
                    {
                        *f = next;
                    }
                } else {
                    let _ = f.set_len(0);
                }
            }
            let _ = f.write_all(line.as_bytes());
            let _ = f.flush();
        }
    }
    fn flush(&self) {
        if let Ok(mut f) = self.file.lock() {
            let _ = f.flush();
        }
    }
}
pub fn init() -> std::io::Result<PathBuf> {
    let exe = std::env::current_exe()?;
    let portable = exe.parent().unwrap().join("nextgen-studio.log");
    let fallback = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_STATE_HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir)
                .join(".local/state")
        })
        .join("NextGenStudio")
        .join("nextgen-studio.log");
    for p in [portable, fallback] {
        if let Some(parent) = p.parent() {
            let _ = fs::create_dir_all(parent);
        }
        // Limit to the current log plus one rotated file (2 MiB each at startup).
        if fs::metadata(&p).is_ok_and(|m| m.len() > 2 * 1024 * 1024) {
            let old = p.with_extension("log.1");
            let _ = fs::remove_file(&old);
            let _ = fs::rename(&p, &old);
        }
        if let Ok(f) = OpenOptions::new().create(true).append(true).open(&p) {
            let logger = Box::leak(Box::new(Logger {
                file: Mutex::new(f),
                path: p.clone(),
            }));
            let _ = log::set_logger(logger);
            log::set_max_level(LevelFilter::Info);
            std::panic::set_hook(Box::new(|info| log::error!("panic: {info}")));
            log::info!(
                "NextGen Studio {} | Debug={} | log={}",
                env!("CARGO_PKG_VERSION"),
                cfg!(debug_assertions),
                p.display()
            );
            return Ok(p);
        }
    }
    Err(std::io::Error::other("Não foi possível criar o log"))
}
