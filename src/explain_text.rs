//! Educational text about secure deletion limits.

pub fn explain_markdown() -> &'static str {
    r#"# Why "secure delete" is hard on modern storage

## The old promise

On a classic spinning HDD with a simple filesystem (e.g. ext3/ext4 without exotic
features), overwriting a file's data blocks several times and then unlinking the
name made forensic recovery of *that file's contents from those blocks* much harder.

## Why SSDs and NVMe break that promise

- **Wear-leveling**: the flash translation layer (FTL) maps logical LBAs to
  changing physical pages. A write to the "same" LBA may land on a different page;
  the old page can remain until garbage-collected.
- **TRIM/DISCARD**: tells the device pages are unused, but does not guarantee
  immediate physical erase visible to you, and behavior varies by firmware.
- **Over-provisioning**: spare pages hold copies you cannot address from the OS.

**Conclusion:** multi-pass overwrite of a file on SSD/NVMe is **not** a reliable
guarantee that the plaintext is physically gone.

## Copy-on-write filesystems (Btrfs, ZFS, and friends)

CoW means an "overwrite" allocates **new** extents. Old extents may remain until
balance/GC, and **snapshots** and send/receive clones keep historical data.
Secure overwrite of a path is largely meaningless; destroy keys / datasets instead.

## Journaling, compression, encryption, backups

- **Journals** may contain fragments of recent writes.
- **Compression/dedup** can leave logical copies elsewhere.
- **Full-disk encryption**: the right recovery strategy is often destroying or
  forgetting the key (crypto-shredding), not overwriting one file.
- **Backups, RAID, cloud sync, thumbnails, editor swap files** retain copies
  outside the path you shred.

## What fileshred will and will not claim

- On **HDD + non-CoW** media it may perform best-effort multi-pass overwrite
  (via `shred` or built-in) then unlink — still **not** a mathematical guarantee.
- On **SSD / CoW / network / unknown** it **refuses** a guaranteed wipe unless you
  pass `--i-understand` / `--force`, and will still only unlink or best-effort
  overwrite while printing loud warnings.
- It will **never** print "securely erased from physical media" when the stack
  cannot guarantee that.

## Better approaches when you need real assurance

1. Encrypt disks from first boot; destroy keys when retiring data.
2. Use vendor secure-erase / nvme format with cryptographic erase where trusted.
3. Physically destroy media for highest assurance.
4. Treat backups and replicas as part of the threat model.

This tool exists to be **honest**, not comforting.
"#
}
