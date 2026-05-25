---
name: Itemize new config settings in release notes
description: Config drift: auto-created default config.toml is not updated when new settings land, so each release must list new config keys explicitly for discoverability
type: project
---

When a release adds a new config setting, the release notes must itemize it (name, type, default, what it does).

**Why:** weld auto-creates a commented `config.toml` template on first launch only. Existing users never get new settings appended to their file — we can't safely rewrite user-edited files. Missing keys fall back to `#[serde(default)]` at runtime, so old configs don't break, but users won't *discover* new settings by looking at their existing file.

**How to apply:** Any PR that adds a field to `weld-tui/src/config.rs::Config` (and therefore a line to `weld-tui/src/default_config.toml`) should include a release-notes bullet. When cutting a release, double-check the config diff against the notes.
