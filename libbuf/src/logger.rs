use anyhow::{Context, Result};
use chrono::Local;
use fern::Dispatch;
use log::LevelFilter;
use std::path::PathBuf;

fn home_dir() -> Option<PathBuf> {
    if let Ok(h) = std::env::var("HOME") {
        return Some(PathBuf::from(h));
    }
    if let Ok(h) = std::env::var("USERPROFILE") {
        return Some(PathBuf::from(h));
    }
    None
}

pub fn log_path() -> Option<PathBuf> {
    let now = Local::now();
    // Colons are illegal in Windows filenames
    #[cfg(windows)]
    let filename = now.format("buf-%m-%d-%y-%H-%M-%S.log").to_string();
    #[cfg(not(windows))]
    let filename = now.format("buf-%m-%d-%y-%H:%M:%S.log").to_string();
    home_dir().map(|d| d.join(filename))
}

pub fn init(enabled: bool, verbose: bool) -> Result<Option<PathBuf>> {
    let level = if verbose { LevelFilter::Debug } else { LevelFilter::Info };

    let formatter =
        |out: fern::FormatCallback, message: &std::fmt::Arguments, record: &log::Record| {
            out.finish(format_args!(
                "[{timestamp}] [{level:<5}] [{target}] {message}",
                timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f"),
                level = record.level(),
                target = record.target(),
                message = message,
            ))
        };

    if !enabled {
        Dispatch::new()
            .format(formatter)
            .level(LevelFilter::Warn)
            .chain(std::io::stderr())
            .apply()
            .context("Failed to initialise stderr logger")?;
        return Ok(None);
    }

    let path = match log_path() {
        Some(p) => p,
        None => {
            // HOME/USERPROFILE unset; can happen when running as SYSTEM after UAC elevation 
            eprintln!("Warning: could not determine home directory, logging to stderr only");
            Dispatch::new()
                .format(formatter)
                .level(level)
                .chain(std::io::stderr())
                .apply()
                .context("Failed to initialise stderr logger")?;
            return Ok(None);
        }
    };

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Could not create log directory: {}", parent.display()))?;
    }

    let log_file = fern::log_file(&path)
        .with_context(|| format!("Could not open log file: {}", path.display()))?;

    Dispatch::new()
        .format(formatter)
        .chain(Dispatch::new().level(level).chain(log_file))
        .chain(Dispatch::new().level(LevelFilter::Warn).chain(std::io::stderr()))
        .apply()
        .context("Failed to initialise logger")?;

    log::info!("buf logging started, file: {}", path.display());
    log::info!("Log level: {}", level);

    Ok(Some(path))
}
