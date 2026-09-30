# Changelog

## Unreleased

### Fixed
- Overwrite never follows a symlink: it used to overwrite the link's *target* and
  then unlink only the link, destroying a file the user did not name. A symlink is
  now refused for overwrite (`--unlink-only` still removes just the link).
- The built-in random pass ignored `/dev/urandom` read errors and could write stale
  buffer contents; it now opens `/dev/urandom` once and fails loudly. The predictable
  "weak fallback" generator is removed.
- `--passes 0` is rejected instead of reporting a successful overwrite that wrote nothing.

### Tests
- Overwrite contents/length, symlink refusal, zero passes, and link-only unlink.

## [0.1.0] - 2026-09-21

### Added
- `inspect`, `explain`, `delete`, `wipe-free-space`, `completions`
- Storage classification (HDD/SSD/CoW/network)
- Refusal without `--i-understand`/`--force` on non-guaranteed media
- Prefer coreutils `shred` when available
- Honest `guaranteed_physical_erasure: false` in all outputs
