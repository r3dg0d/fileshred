# Security Policy

## Product security honesty

fileshred is a **safety-critical UX** tool: incorrect claims about erasure are a security failure.

- Never report guaranteed physical erasure on SSD/CoW/unknown stacks.
- `--force` must still document ineffectiveness.
- Free-space wipe is optional and ineffective on many modern devices.

## Vulnerability reporting

Contact **r3dg0d** privately for issues that could cause users to believe data is gone when it is not, or for memory-safety bugs in the wipe path.
