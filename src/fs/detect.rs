use super::classify::{advise, DeleteAdvice, StorageClass};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathInspection {
    pub path: PathBuf,
    pub exists: bool,
    pub is_file: bool,
    pub is_dir: bool,
    pub size_bytes: Option<u64>,
    pub filesystem_type: Option<String>,
    pub mount_point: Option<String>,
    pub mount_options: Vec<String>,
    pub source_device: Option<String>,
    pub storage_class: StorageClass,
    pub advice: DeleteAdvice,
    pub rotational: Option<bool>,
    pub notes: Vec<String>,
}

pub fn inspect_path(path: &Path) -> Result<PathInspection> {
    let canon = if path.exists() {
        path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
    } else {
        path.to_path_buf()
    };

    let meta = std::fs::metadata(path).ok();
    let exists = meta.is_some();
    let is_file = meta.as_ref().map(|m| m.is_file()).unwrap_or(false);
    let is_dir = meta.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    let size_bytes = meta.as_ref().filter(|m| m.is_file()).map(|m| m.len());

    let mount = findmnt(&canon).or_else(|| findmnt(path));
    let filesystem_type = mount.as_ref().map(|m| m.fstype.clone());
    let mount_point = mount.as_ref().map(|m| m.target.clone());
    let mount_options = mount
        .as_ref()
        .map(|m| {
            m.options
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let source_device = mount.as_ref().map(|m| m.source.clone());

    let rotational = source_device
        .as_ref()
        .and_then(|s| detect_rotational(s));

    let (storage_class, mut notes) = classify_storage(
        filesystem_type.as_deref(),
        &mount_options,
        rotational,
        source_device.as_deref(),
    );

    let advice = advise(storage_class, is_file);

    if mount_options.iter().any(|o| o == "ro") {
        notes.push("Mount is read-only.".into());
    }

    Ok(PathInspection {
        path: path.to_path_buf(),
        exists,
        is_file,
        is_dir,
        size_bytes,
        filesystem_type,
        mount_point,
        mount_options,
        source_device,
        storage_class,
        advice,
        rotational,
        notes,
    })
}

#[derive(Debug, Clone)]
struct MountInfo {
    target: String,
    source: String,
    fstype: String,
    options: String,
}

fn findmnt(path: &Path) -> Option<MountInfo> {
    let out = Command::new("findmnt")
        .args(["-n", "-o", "TARGET,SOURCE,FSTYPE,OPTIONS", "-T"])
        .arg(path)
        .output()
        .ok()?;
    if !out.status.success() {
        return parse_proc_mounts(path);
    }
    let line = String::from_utf8_lossy(&out.stdout);
    let line = line.lines().next()?.trim();
    parse_findmnt_line(line).or_else(|| parse_proc_mounts(path))
}

fn parse_findmnt_line(line: &str) -> Option<MountInfo> {
    // findmnt default separates by spaces but SOURCE may contain spaces rarely.
    // Prefer tab if present.
    let parts: Vec<&str> = if line.contains('\t') {
        line.split('\t').collect()
    } else {
        line.split_whitespace().collect()
    };
    if parts.len() < 4 {
        // TARGET SOURCE FSTYPE OPTIONS — options may have been split
        if parts.len() >= 3 {
            return Some(MountInfo {
                target: parts[0].into(),
                source: parts[1].into(),
                fstype: parts[2].into(),
                options: parts.get(3).unwrap_or(&"").to_string(),
            });
        }
        return None;
    }
    Some(MountInfo {
        target: parts[0].into(),
        source: parts[1].into(),
        fstype: parts[2].into(),
        options: parts[3..].join(" "),
    })
}

fn parse_proc_mounts(path: &Path) -> Option<MountInfo> {
    let text = std::fs::read_to_string("/proc/mounts").ok()?;
    let path_str = path.to_string_lossy();
    let mut best: Option<MountInfo> = None;
    let mut best_len = 0usize;
    for line in text.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 4 {
            continue;
        }
        let target = parts[1];
        if path_str.starts_with(target) && target.len() >= best_len {
            best_len = target.len();
            best = Some(MountInfo {
                target: target.into(),
                source: parts[0].into(),
                fstype: parts[2].into(),
                options: parts[3].into(),
            });
        }
    }
    best
}

fn detect_rotational(source: &str) -> Option<bool> {
    // source like /dev/sda1 or /dev/mapper/vg-lv or UUID=
    let dev = Path::new(source);
    let name = dev.file_name()?.to_string_lossy().to_string();
    // strip partition digits for sdX, nvme0n1p2 -> nvme0n1
    let base = block_base(&name);
    let rot_path = Path::new("/sys/block").join(&base).join("queue/rotational");
    let val = std::fs::read_to_string(rot_path).ok()?;
    Some(val.trim() == "1")
}

pub fn block_base(name: &str) -> String {
    if name.starts_with("nvme") {
        // nvme0n1p2 -> nvme0n1
        if let Some(idx) = name.rfind('p') {
            let after = &name[idx + 1..];
            if !after.is_empty() && after.chars().all(|c| c.is_ascii_digit()) {
                return name[..idx].to_string();
            }
        }
        return name.to_string();
    }
    // sda1 -> sda, vda2 -> vda, mmcblk0p1 -> mmcblk0
    if name.starts_with("mmcblk") {
        if let Some(idx) = name.rfind('p') {
            let after = &name[idx + 1..];
            if !after.is_empty() && after.chars().all(|c| c.is_ascii_digit()) {
                return name[..idx].to_string();
            }
        }
        return name.to_string();
    }
    name
        .trim_end_matches(|c: char| c.is_ascii_digit())
        .to_string()
}

fn classify_storage(
    fstype: Option<&str>,
    options: &[String],
    rotational: Option<bool>,
    source: Option<&str>,
) -> (StorageClass, Vec<String>) {
    let mut notes = Vec::new();
    let fs = fstype.unwrap_or("unknown").to_ascii_lowercase();

    if fs.contains("btrfs") {
        notes.push("Btrfs is CoW; overwrites do not reliably destroy old extents.".into());
        return (StorageClass::CowFilesystem, notes);
    }
    if fs.contains("zfs") {
        notes.push("ZFS is CoW with datasets/snapshots; secure overwrite is not meaningful.".into());
        return (StorageClass::CowFilesystem, notes);
    }
    if fs == "nfs" || fs == "nfs4" || fs == "cifs" || fs == "smb3" || fs == "fuse" || fs.starts_with("fuse.") {
        notes.push("Network/FUSE filesystem: physical media is remote or abstracted.".into());
        return (StorageClass::NetworkOrVirtual, notes);
    }
    if fs == "tmpfs" || fs == "ramfs" || fs == "overlay" || fs == "overlayfs" {
        notes.push("Volatile or layered FS; underlying layers may retain data.".into());
        return (StorageClass::NetworkOrVirtual, notes);
    }

    if options.iter().any(|o| o.starts_with("subvol") || o == "compress" || o.starts_with("compress=")) {
        notes.push("Mount options suggest CoW/compression features.".into());
    }

    match rotational {
        Some(true) => {
            notes.push("Block device reports rotational=1 (classic HDD behavior likely).".into());
            if fs == "ext4" || fs == "ext3" || fs == "xfs" || fs == "ext2" {
                (StorageClass::HddTraditional, notes)
            } else {
                notes.push(format!("Filesystem {fs}: overwrite best-effort only."));
                (StorageClass::HddTraditional, notes)
            }
        }
        Some(false) => {
            notes.push("Block device reports rotational=0 (SSD/NVMe). TRIM/wear-leveling defeat overwrite guarantees.".into());
            (StorageClass::SsdFlash, notes)
        }
        None => {
            if let Some(src) = source {
                if src.contains("nvme") {
                    notes.push("Device name suggests NVMe.".into());
                    return (StorageClass::SsdFlash, notes);
                }
            }
            notes.push("Could not determine rotational flag; assuming uncertain.".into());
            (StorageClass::Unknown, notes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_base_names() {
        assert_eq!(block_base("sda1"), "sda");
        assert_eq!(block_base("nvme0n1p3"), "nvme0n1");
        assert_eq!(block_base("mmcblk0p1"), "mmcblk0");
    }

    #[test]
    fn parse_findmnt() {
        let m = parse_findmnt_line("/ /dev/sda1 ext4 rw,relatime").unwrap();
        assert_eq!(m.fstype, "ext4");
    }
}
