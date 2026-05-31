/*
    buf - Tool for flashing USB drives across platforms
    Copyright (C) 2026 Bryson Kelly

    This program is free software: you can redistribute it and/or modify
    it under the terms of the GNU General Public License as published by
    the Free Software Foundation, either version 3 of the License, or
    (at your option) any later version.

    This program is distributed in the hope that it will be useful,
    but WITHOUT ANY WARRANTY; without even the implied warranty of
    MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
    GNU General Public License for more details.

    You should have received a copy of the GNU General Public License
    along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */


use anyhow::{bail, Context, Result};
use clap::{ArgAction, Parser};
use log::{debug, error, info, warn};

#[derive(Parser, Debug)]
#[command(
    name       = "buf",
    version    = "0.1.1",
    author     = "Bryson Kelly",
    about      = "A fast, safe bootable USB image flasher",
    long_about = None,
    styles     = clap_styles(),
)]
struct Cli {
    #[arg(
        short = 's',
        long = "source",
        value_name = "FILE",
        help = "Source ISO/IMG file to flash"
    )]
    source: Option<String>,

    // I cannot stress enough, I *fucking* HATE the way windows does drive naming...
    // `\\.\PhysicalDriveN`.... What a stupid convention
    #[arg(
        short = 't',
        long = "target",
        value_name = "DEVICE",
        help = "Target block device (e.g. /dev/sdb, /dev/disk2, or \\\\.\\PhysicalDrive1)"
    )]
    target: Option<String>,

    #[arg(
        short = 'l',
        long = "list",
        action = ArgAction::SetTrue,
        help = "List storage devices and exit"
    )]
    list: bool,

    #[arg(
        short = 'b',
        long = "block-size",
        value_name = "SIZE",
        default_value = "32MiB",
        help = "Write block size (default: 32MiB)"
    )]
    block_size: String,

    #[arg(
        long = "offset",
        value_name = "BYTES",
        default_value_t = 0,
        help = "Start writing at this byte offset into the target"
    )]
    offset: u64,

    #[arg(
        short = 'f',
        long = "force",
        action = ArgAction::SetTrue,
        help = "Skip the confirmation prompt"
    )]
    force: bool,

    #[arg(
        long = "dry-run",
        action = ArgAction::SetTrue,
        help = "Validate everything without writing any data"
    )]
    dry_run: bool,

    #[arg(
        long = "no-logging",
        short_alias = 'n',
        action = ArgAction::SetTrue,
        help = "Disable log file creation"
    )]
    no_logging: bool,

    #[arg(
        short = 'v',
        long = "verbose",
        action = ArgAction::SetTrue,
        help = "Enable verbose debug logging"
    )]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli) {
        error!("{:#}", e);
        eprintln!("\n  Error: {:#}\n", e);
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    if cli.list {
        let devices = libbuf::list_drives()?;
        libbuf::print_device_table(&devices);
        return Ok(());
    }

    let log_path = libbuf::init_logger(!cli.no_logging, cli.verbose).unwrap_or_else(|e| {
        eprintln!("Warning: could not initialise logger: {}", e);
        None
    });

    if let Some(ref path) = log_path {
        println!("  Logging to: {}", path.display());
    }

    info!("buf started");
    debug!("Parsed CLI args: {:?}", cli);

    let source = cli
        .source
        .ok_or_else(|| anyhow::anyhow!("--source/-s is required. Use --help for usage."))?;

    let target = cli
        .target
        .ok_or_else(|| anyhow::anyhow!("--target/-t is required. Use --help for usage."))?;

    // Resolve to absolute before elevation. UAC relaunch via cmd.exe resets the
    // working directory to System32, so a relative path would not resolve correctly.
    let source = {
        let p = std::path::Path::new(&source);
        if p.is_absolute() {
            source
        } else {
            std::fs::canonicalize(p)
                .with_context(|| format!("Could not resolve source path: {}", source))?
                .to_string_lossy()
                .to_string()
        }
    };

    if !libbuf::is_privileged() {
        warn!("Not running as root/Administrator");
        let mut argv = vec![
            "--source".to_string(), source.clone(),
            "--target".to_string(), target.clone(),
        ];
        if cli.block_size != "32MiB" {
            argv.extend(["--block-size".to_string(), cli.block_size.clone()]);
        }
        if cli.offset != 0 {
            argv.extend(["--offset".to_string(), cli.offset.to_string()]);
        }
        if cli.force     { argv.push("--force".to_string()); }
        if cli.dry_run   { argv.push("--dry-run".to_string()); }
        if cli.no_logging { argv.push("--no-logging".to_string()); }
        if cli.verbose   { argv.push("--verbose".to_string()); }
        libbuf::elevate_or_warn(&argv)?;
    }

    let block_size = parse_size(&cli.block_size)
        .map_err(|e| anyhow::anyhow!("Invalid --block-size '{}': {}", cli.block_size, e))?;

    if block_size == 0 {
        bail!("Block size must be greater than zero");
    }

    info!("Block size resolved to {} bytes", block_size);

    let params = libbuf::WriteParams {
        source: source.clone(),
        target: target.clone(),
        block_size,
        offset: cli.offset,
    };

    println!("\n  Validating source and target...");
    let (source_size, target_file) = libbuf::validate(&params)?;
    println!("  Validation passed.");
    info!("Validation passed, source size {} bytes", source_size);

    if cli.dry_run {
        println!("\n  --dry-run: all checks passed. Nothing was written.\n");
        info!("Dry-run complete, exiting without writing");
        return Ok(());
    }

    if !cli.force {
        confirm(&source, &target, source_size)?;
    } else {
        warn!("--force set, skipping confirmation prompt");
        println!("\n  --force: skipping confirmation, {} -> {}", source, target);
    }

    println!("\n  Writing {} -> {}...\n", source, target);
    info!("Starting write: {} -> {}", source, target);

    libbuf::write(&params, source_size, target_file)?;

    info!("Write completed successfully");
    println!("Write completed successfully.");

    Ok(())
}

fn confirm(source: &str, target: &str, source_size: u64) -> Result<()> {
    use libbuf::list::human_bytes;
    use std::io::{self, Write as _};

    print!(
        "\n  Flash {} ({}) to {}? [y/N]: ",
        source,
        human_bytes(source_size),
        target
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim().to_ascii_lowercase();

    if trimmed != "y" {
        info!("User declined (input: {:?})", trimmed);
        bail!("Aborted by user.");
    }

    info!("User confirmed write");
    Ok(())
}

// Accepts plain bytes or suffixes: K/KB/KiB, M/MB/MiB, G/GB/GiB (case-insensitive, powers of 1024)
fn parse_size(s: &str) -> Result<usize> {
    let s = s.trim();
    let split_pos = s.find(|c: char| !c.is_ascii_digit()).unwrap_or(s.len());
    let (num_str, suffix) = s.split_at(split_pos);

    if num_str.is_empty() {
        bail!("No numeric value found in '{}'", s);
    }

    let num: u64 = num_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Could not parse '{}' as a number", num_str))?;

    let multiplier: u64 = match suffix.to_ascii_uppercase().as_str() {
        "" | "B" => 1,
        "K" | "KB" | "KIB" => 1024,
        "M" | "MB" | "MIB" => 1024 * 1024,
        "G" | "GB" | "GIB" => 1024 * 1024 * 1024,
        other => bail!("Unknown size suffix: '{}'", other),
    };

    let bytes = num
        .checked_mul(multiplier)
        .ok_or_else(|| anyhow::anyhow!("Size overflows u64: {}", s))?;

    if bytes > usize::MAX as u64 {
        bail!("Block size {} is too large for this platform", s);
    }

    Ok(bytes as usize)
}

fn clap_styles() -> clap::builder::Styles {
    use clap::builder::styling::{AnsiColor, Effects, Styles};
    Styles::styled()
        .header(AnsiColor::BrightCyan.on_default() | Effects::BOLD)
        .usage(AnsiColor::BrightCyan.on_default() | Effects::BOLD)
        .literal(AnsiColor::BrightGreen.on_default())
        .placeholder(AnsiColor::BrightYellow.on_default())
        .error(AnsiColor::BrightRed.on_default() | Effects::BOLD)
        .valid(AnsiColor::BrightGreen.on_default())
        .invalid(AnsiColor::BrightRed.on_default())
}

#[cfg(test)]
mod tests {
    use super::parse_size;

    #[test]
    fn test_parse_size_plain() {
        assert_eq!(parse_size("4096").unwrap(), 4096);
        assert_eq!(parse_size("0").unwrap(), 0);
        assert_eq!(parse_size("1").unwrap(), 1);
    }

    #[test]
    fn test_parse_size_kib() {
        assert_eq!(parse_size("1KiB").unwrap(), 1024);
        assert_eq!(parse_size("4KB").unwrap(), 4 * 1024);
        assert_eq!(parse_size("4K").unwrap(), 4 * 1024);
        assert_eq!(parse_size("4kib").unwrap(), 4 * 1024);
    }

    #[test]
    fn test_parse_size_mib() {
        assert_eq!(parse_size("32MiB").unwrap(), 32 * 1024 * 1024);
        assert_eq!(parse_size("32MB").unwrap(), 32 * 1024 * 1024);
        assert_eq!(parse_size("1M").unwrap(), 1024 * 1024);
    }

    #[test]
    fn test_parse_size_gib() {
        assert_eq!(parse_size("1GiB").unwrap(), 1024 * 1024 * 1024);
        assert_eq!(parse_size("2GB").unwrap(), 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_parse_size_bad_suffix() {
        assert!(parse_size("1TiB").is_err());
        assert!(parse_size("abc").is_err());
        assert!(parse_size("MiB").is_err());
    }
}
