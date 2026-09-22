use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StorageClass {
    HddTraditional,
    SsdFlash,
    CowFilesystem,
    NetworkOrVirtual,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeleteAdvice {
    /// Multi-pass overwrite may be meaningful (classic HDD + non-CoW).
    OverwriteMeaningful,
    /// Overwrite is theater; refuse guaranteed wipe.
    OverwriteNotMeaningful,
    /// Need user force / acknowledgment.
    RequireAcknowledgement,
    /// Path unsuitable (dir, missing, etc.)
    NotApplicable,
}

pub fn advise(class: StorageClass, is_file: bool) -> DeleteAdvice {
    if !is_file {
        return DeleteAdvice::NotApplicable;
    }
    match class {
        StorageClass::HddTraditional => DeleteAdvice::OverwriteMeaningful,
        StorageClass::SsdFlash
        | StorageClass::CowFilesystem
        | StorageClass::NetworkOrVirtual
        | StorageClass::Unknown => DeleteAdvice::RequireAcknowledgement,
    }
}

impl StorageClass {
    pub fn explain(self) -> &'static str {
        match self {
            Self::HddTraditional => {
                "Traditional rotational disk with non-CoW FS: multi-pass overwrite can reduce recoverability of current file contents, but journaling, remapped sectors, and prior copies may remain."
            }
            Self::SsdFlash => {
                "SSD/NVMe: wear-leveling and TRIM mean overwriting a file's logical blocks does not guarantee physical erasure."
            }
            Self::CowFilesystem => {
                "Copy-on-write filesystem (Btrfs/ZFS/etc.): old extents can remain until GC/balance; snapshots retain data."
            }
            Self::NetworkOrVirtual => {
                "Remote or virtualized storage: local overwrite does not control remote retention, snapshots, or backups."
            }
            Self::Unknown => {
                "Storage characteristics unknown: treat secure delete as unproven."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hdd_file_ok() {
        assert_eq!(
            advise(StorageClass::HddTraditional, true),
            DeleteAdvice::OverwriteMeaningful
        );
    }

    #[test]
    fn ssd_requires_ack() {
        assert_eq!(
            advise(StorageClass::SsdFlash, true),
            DeleteAdvice::RequireAcknowledgement
        );
    }

    #[test]
    fn dir_not_applicable() {
        assert_eq!(
            advise(StorageClass::HddTraditional, false),
            DeleteAdvice::NotApplicable
        );
    }
}
