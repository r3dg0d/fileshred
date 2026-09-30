use crate::cli::Cli;
use crate::commands::opts_from;
use crate::exit_codes;
use crate::fs::{self, StorageClass};
use crate::output::{self, print_json};
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs::{self as stdfs, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct WipeFreeOut {
    path: PathBuf,
    storage_class: StorageClass,
    performed: bool,
    dry_run: bool,
    message: String,
    guaranteed_physical_erasure: bool,
}

pub fn run(cli: &Cli, path: PathBuf, i_understand: bool, force: bool) -> Result<i32> {
    let opts = opts_from(cli);
    let insp = fs::inspect_path(&path)?;
    let ack = i_understand || force;

    if !ack {
        let msg = "refusing wipe-free-space without --i-understand or --force (often ineffective on SSD/CoW)".to_string();
        output::warn(opts, &msg);
        if opts.json {
            print_json(
                opts,
                &WipeFreeOut {
                    path,
                    storage_class: insp.storage_class,
                    performed: false,
                    dry_run: cli.dry_run,
                    message: msg,
                    guaranteed_physical_erasure: false,
                },
            )?;
        }
        return Ok(exit_codes::REFUSED);
    }

    output::warn(
        opts,
        "Free-space wiping does NOT securely erase deleted file contents on SSD/CoW/journaled systems.",
    );

    if cli.dry_run {
        let msg = format!(
            "dry-run: would create large temp file(s) under {} until ENOSPC, then remove",
            path.display()
        );
        emit(
            opts,
            &WipeFreeOut {
                path,
                storage_class: insp.storage_class,
                performed: false,
                dry_run: true,
                message: msg,
                guaranteed_physical_erasure: false,
            },
        )?;
        return Ok(0);
    }

    let target_dir = if insp.is_dir {
        path.clone()
    } else {
        path.parent().unwrap_or(Path::new(".")).to_path_buf()
    };

    fill_until_full(&target_dir)?;

    emit(
        opts,
        &WipeFreeOut {
            path,
            storage_class: insp.storage_class,
            performed: true,
            dry_run: false,
            message:
                "free-space fill completed and temp files removed; physical erasure NOT guaranteed"
                    .into(),
            guaranteed_physical_erasure: false,
        },
    )?;
    Ok(0)
}

fn fill_until_full(dir: &Path) -> Result<()> {
    let mut temps = Vec::new();
    let chunk = vec![0u8; 1024 * 1024]; // 1 MiB zeros
    let mut idx = 0u32;
    loop {
        let p = dir.join(format!(".fileshred-freewipe-{idx}.tmp"));
        idx += 1;
        let mut file = match OpenOptions::new().write(true).create_new(true).open(&p) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(e) => {
                cleanup(&temps);
                return Err(e).context("create temp");
            }
        };
        temps.push(p.clone());
        loop {
            match file.write_all(&chunk) {
                Ok(()) => {},
                Err(e) if e.raw_os_error() == Some(28) /* ENOSPC */ => {
                    let _ = file.sync_all();
                    cleanup(&temps);
                    return Ok(());
                }
                Err(e) => {
                    cleanup(&temps);
                    return Err(e).context("write temp");
                }
            }
            // soft cap per file 256 MiB then next file
            if file.metadata()?.len() >= 256 * 1024 * 1024 {
                break;
            }
        }
        if idx > 10_000 {
            cleanup(&temps);
            break;
        }
    }
    cleanup(&temps);
    Ok(())
}

fn cleanup(temps: &[PathBuf]) {
    for p in temps {
        let _ = stdfs::remove_file(p);
    }
}

fn emit(opts: output::OutputOpts, out: &WipeFreeOut) -> Result<()> {
    if opts.json {
        print_json(opts, out)?;
    } else if !opts.quiet {
        println!("{}", out.message);
        println!("guaranteed_physical_erasure: false");
    }
    Ok(())
}
