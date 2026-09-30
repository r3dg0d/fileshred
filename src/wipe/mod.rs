//! Best-effort overwrite helpers.

use anyhow::{bail, Context, Result};
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
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

    if passes == 0 {
        bail!("--passes must be at least 1");
    }
    // symlink_metadata: do not follow. Overwriting through a link would destroy
    // the target while only the link is unlinked, i.e. a file the user never named.
    let meta = fs::symlink_metadata(path).with_context(|| format!("stat {}", path.display()))?;
    if meta.file_type().is_symlink() {
        bail!(
            "refusing: {} is a symlink; overwrite the file it points to explicitly, or use --unlink-only to remove just the link",
            path.display()
        );
    }
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
    let mut urandom =
        File::open("/dev/urandom").context("open /dev/urandom for the random pass")?;

    let mut buf = vec![0u8; 64 * 1024];
    for pass in 0..passes {
        file.seek(SeekFrom::Start(0))?;
        // random, then ones, then zeros; the last of three passes is zeros.
        let pattern = pass % 3;
        let mut remaining = len;
        while remaining > 0 {
            let chunk = std::cmp::min(remaining, buf.len() as u64) as usize;
            match pattern {
                0 => urandom
                    .read_exact(&mut buf[..chunk])
                    .context("reading /dev/urandom")?,
                1 => buf[..chunk].fill(0xFF),
                _ => buf[..chunk].fill(0x00),
            }
            file.write_all(&buf[..chunk])?;
            remaining -= chunk as u64;
        }
        file.sync_all()?;
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    fn temp(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("fileshred-test-{}-{name}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn builtin_overwrite_replaces_contents_and_unlinks() {
        let dir = temp("overwrite");
        let f = dir.join("secret.txt");
        fs::write(&f, vec![b'A'; 200_000]).unwrap();
        // Keep a second handle to see what is on disk after the overwrite pass.
        builtin_overwrite(&f, 3).unwrap();
        let after = fs::read(&f).unwrap();
        assert_eq!(after.len(), 200_000, "length is preserved");
        assert!(after.iter().all(|&b| b == 0), "three passes end on zeros");
        let r = overwrite_and_unlink(&f, 1, false, false).unwrap();
        assert!(r.unlinked && !f.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn refuses_a_symlink_and_leaves_the_target_untouched() {
        let dir = temp("symlink");
        let target = dir.join("precious.txt");
        fs::write(&target, b"keep me").unwrap();
        let link = dir.join("link");
        symlink(&target, &link).unwrap();
        let err = overwrite_and_unlink(&link, 3, false, false).unwrap_err();
        assert!(err.to_string().contains("symlink"), "{err}");
        assert_eq!(fs::read(&target).unwrap(), b"keep me");
        assert!(link.exists());
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn zero_passes_is_rejected_instead_of_reporting_success() {
        let dir = temp("zero");
        let f = dir.join("f");
        fs::write(&f, b"data").unwrap();
        assert!(overwrite_and_unlink(&f, 0, false, false).is_err());
        assert_eq!(fs::read(&f).unwrap(), b"data");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn unlink_only_removes_just_the_link() {
        let dir = temp("unlinklink");
        let target = dir.join("t");
        fs::write(&target, b"x").unwrap();
        let link = dir.join("l");
        symlink(&target, &link).unwrap();
        unlink_only(&link, false).unwrap();
        assert!(!link.exists() && target.exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
