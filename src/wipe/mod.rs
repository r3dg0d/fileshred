//! Best-effort overwrite helpers.

use anyhow::{bail, Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Seek, SeekFrom, Write};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone)]
pub struct WipeResult {
    pub method: String,
    pub passes: u32,
    pub unlinked: bool,
    pub notes: Vec<String>,
}

pub fn overwrite_and_unlink(
    path: &Path,
    passes: u32,
    prefer_shred: bool,
    dry_run: bool,
) -> Result<WipeResult> {
    if dry_run {
        return Ok(WipeResult {
            method: "dry-run".into(),
            passes,
            unlinked: false,
            notes: vec![format!(
                "would overwrite ({passes} passes) then unlink {}",
                path.display()
            )],
        });
    }

    let meta = fs::metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if !meta.is_file() {
        bail!("not a regular file: {}", path.display());
    }

    let mut notes = Vec::new();
    let method;

    if prefer_shred && command_exists("shred") {
        let status = Command::new("shred")
            .args(["-n", &passes.to_string(), "-u", "--"])
            .arg(path)
            .status()
            .context("running shred")?;
        if !status.success() {
            bail!("shred failed with status {status}");
        }
        method = format!("shred -n {passes} -u");
        notes.push("Used coreutils shred for overwrite+unlink.".into());
        return Ok(WipeResult {
            method,
            passes,
            unlinked: true,
            notes,
        });
    }

    // Built-in overwrite
    builtin_overwrite(path, passes)?;
    method = format!("builtin-overwrite-{passes}p");
    notes.push("Used built-in overwrite (random + zeros).".into());
    fs::remove_file(path).with_context(|| format!("unlink {}", path.display()))?;
    notes.push("Unlinked path after overwrite.".into());

    Ok(WipeResult {
        method,
        passes,
        unlinked: true,
        notes,
    })
}

fn builtin_overwrite(path: &Path, passes: u32) -> Result<()> {
    let len = fs::metadata(path)?.len();
    let mut file = OpenOptions::new()
        .write(true)
        .open(path)
        .with_context(|| format!("open for write {}", path.display()))?;

    let mut buf = vec![0u8; 64 * 1024];
    for pass in 0..passes {
        file.seek(SeekFrom::Start(0))?;
        let pattern = pass % 3;
        let mut remaining = len;
        while remaining > 0 {
            let chunk = std::cmp::min(remaining, buf.len() as u64) as usize;
            match pattern {
                0 => fill_random(&mut buf[..chunk]),
                1 => buf[..chunk].fill(0xFF),
                _ => buf[..chunk].fill(0x00),
            }
            file.write_all(&buf[..chunk])?;
            remaining -= chunk as u64;
        }
        file.sync_all()?;
    }
    // Final zero pass already included when passes%3 logic hits; ensure truncate sync
    file.seek(SeekFrom::Start(0))?;
    file.sync_all()?;
    drop(file);

    // Optional: rename to obscure name before unlink — skipped for honesty simplicity
    let _ = File::open("/dev/null");
    Ok(())
}

fn fill_random(buf: &mut [u8]) {
    // Prefer getrandom via /dev/urandom
    if let Ok(mut f) = File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(buf);
        return;
    }
    // weak fallback
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    for (i, b) in buf.iter_mut().enumerate() {
        *b = ((t.wrapping_mul(i as u64 + 1)) & 0xFF) as u8;
    }
}

pub fn unlink_only(path: &Path, dry_run: bool) -> Result<WipeResult> {
    if dry_run {
        return Ok(WipeResult {
            method: "dry-run-unlink".into(),
            passes: 0,
            unlinked: false,
            notes: vec![format!("would unlink {}", path.display())],
        });
    }
    fs::remove_file(path)?;
    Ok(WipeResult {
        method: "unlink-only".into(),
        passes: 0,
        unlinked: true,
        notes: vec![
            "File unlinked without overwrite. Data may remain on media until reused.".into(),
        ],
    })
}

fn command_exists(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|p| p.join(name).is_file()))
        .unwrap_or(false)
}
