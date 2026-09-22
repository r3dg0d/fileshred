# fileshred

**Honest secure-delete** — best-effort overwrite with loud, accurate limitations for SSDs, NVMe, TRIM, Btrfs, ZFS, CoW, journaling, encryption, and backups.

Owner: **r3dg0d** · License: **MIT**

> This tool will **never** claim a file is guaranteed physically gone when the storage stack cannot guarantee it.

## Install

```bash
cargo build --release
nix build   # optional
```

## Commands

```bash
fileshred inspect ./secret.txt
fileshred explain
fileshred delete ./secret.txt --dry-run
fileshred delete ./secret.txt                  # HDD/non-CoW: overwrite+unlink
fileshred delete ./secret.txt --i-understand   # SSD/CoW: unlink after ack
fileshred delete ./secret.txt --force          # force overwrite attempt + loud warning
fileshred wipe-free-space /mnt/data --i-understand --dry-run
```

### Global flags

`--json --verbose --quiet --config --dry-run` plus `--help` / `--version`.

Exit codes: `0` ok, `1` error, `3` refused, `5` not found, `130` Ctrl+C.

## Behavior

| Storage class | Default `delete` |
|---------------|------------------|
| HDD + non-CoW (ext4/xfs/…) | Multi-pass overwrite (`shred` if available) then unlink |
| SSD / NVMe | **Refuse** unless `--i-understand` / `--force` |
| Btrfs / ZFS | **Refuse** unless ack |
| Network / FUSE / overlay | **Refuse** unless ack |
| Unknown | **Refuse** unless ack |

`inspect` reports fstype (findmnt/statfs), mount options, rotational sysfs flag, and advice.

## Limitations (mandatory reading)

Run `fileshred explain` or see SECURITY.md. Short version: wear-leveling, TRIM, CoW snapshots, journals, and backups defeat file-level overwrite theater.

## Architecture

```
src/fs/     # detect + classify storage
src/wipe/   # shred or builtin overwrite
src/commands/
```

## License

MIT © r3dg0d
