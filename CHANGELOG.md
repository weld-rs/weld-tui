# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-05-29

Initial release. Weld is a cross-platform TUI diff and merge tool — a
terminal-native alternative to meld, BeyondCompare, and DiffMerge.

### Added
- Side-by-side file diff with line-level alignment
- Word-level inline highlighting within changed lines
- Active block indicator with `J`/`K`/`gg`/`G` navigation
- Block copy operations (`H` copies right→left, `L` copies left→right)
- Undo/redo for copy operations (`u` / `Ctrl+r`), command-pattern based
- Save (`w`) and quit (`q` / `q!`) with dirty-state awareness
- Minimap with diff markers and viewport indicator
- Config file support (framework + `show_minimap`)
- Cross-platform path handling (Windows `USERPROFILE` fallback)
- Release workflow (`workflow_dispatch`) producing prebuilt binaries for
  macOS (arm64, x86_64) and Linux (x86_64, arm64). Windows is stubbed
  pending #42.

[Unreleased]: https://github.com/weld-rs/weld-tui/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/weld-rs/weld-tui/releases/tag/v0.1.0
