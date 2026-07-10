---
name: macos-13 runner queue + deprecation
description: GitHub Actions `macos-13` (Intel) runners have indeterminate queue depth (50+ min waits observed) and are on the deprecation path; for x86_64-apple-darwin builds prefer macos-latest + target
type: project
---

`macos-13` (Intel) runners on GitHub-hosted Actions have unpredictable queue depth. Observed 52+ min wait for a single job with no SLA. `macos-latest` (arm64) starts within seconds. GitHub is also retiring `macos-13`, so pinning to it is borrowed time.

**For x86_64-apple-darwin builds, the future-proof options are:**

1. **Cross-compile from an arm64 runner.** `macos-latest` + `targets: x86_64-apple-darwin` on the `dtolnay/rust-toolchain` action + `cargo build --target x86_64-apple-darwin`. Works for pure-Rust deps. Fast (~3-5 min). Caveat: untestable on the build host.
2. **Universal binary via `lipo`.** Build both targets on `macos-latest`, fuse with `lipo -create`. Single artifact covers both archs.
3. **Drop Intel.** Reasonable for new projects given Apple Silicon adoption.

In weld-tui, x86_64-apple-darwin was stubbed as a no-op job (matching the Windows stub pattern) pending a decision (issue #46). The matrix slot stays visible in the Actions UI without blocking releases.
