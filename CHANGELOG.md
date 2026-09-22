# Changelog

## [0.1.0] - 2026-09-21

### Added
- `inspect`, `explain`, `delete`, `wipe-free-space`, `completions`
- Storage classification (HDD/SSD/CoW/network)
- Refusal without `--i-understand`/`--force` on non-guaranteed media
- Prefer coreutils `shred` when available
- Honest `guaranteed_physical_erasure: false` in all outputs
