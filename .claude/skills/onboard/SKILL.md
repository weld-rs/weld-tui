---
name: onboard
description: Walk a new contributor through prerequisites, bootstrap, and orientation reading for the weld-tui repo. Use when a user runs /onboard or says they're new and need to get set up.
---

# Onboard

Walk a new contributor through everything needed to start contributing to weld-tui. Run steps in order. Pause after each step and let the user confirm before moving on.

## Step 1 — Verify prerequisites

Install the required tools:

- **Rust >= 1.94** — install via [rustup](https://www.rust-lang.org/tools/install) or [mise](https://mise.jdx.dev/).
- **just** — command runner. `brew install just` on macOS; see [just install docs](https://github.com/casey/just) for other platforms.
- **Kingfisher** — secrets scanner used by the pre-commit hook. `brew install kingfisher` on macOS; see [Kingfisher releases](https://github.com/trufflesecurity/kingfisher) elsewhere.

Then run:

```bash
just bootstrap
```

This configures `core.hooksPath` to `.githooks/` (activating the pre-commit hook) and verifies that `cargo` and `kingfisher` are on `PATH`. Idempotent — safe to re-run. Resolve any reported failures before continuing.

## Step 2 — Build and test smoke check

```bash
just check     # cargo fmt + clippy
just test      # cargo test --all
```

Then a quick manual smoke test against any two text files:

```bash
cargo run --bin weld-tui -- <left-file> <right-file>
```

If the TUI launches and renders a side-by-side diff, the environment is good.

## Step 3 — Orientation reading

In order:

- `README.md` — project overview, navigation/merge keybindings, contributor prerequisites.
- `.claude/CLAUDE.md` — project conventions: idiomatic Rust expectations, the `weld-core` vs `weld-tui` crate boundary, fail-loud philosophy.
- `docs/superpowers/specs/2026-04-02-weld-tui-diff-design.md` — design spec for the diff/merge model.
- `docs/superpowers/plans/2026-04-02-weld-phase1-file-diff-mvp.md` — phase 1 implementation plan; useful context for understanding the current scope.

## Step 4 — Team channels

- **GitHub Issues** at [`weld-rs/weld-tui`](https://github.com/weld-rs/weld-tui/issues) — bug reports, feature requests, design discussion.

## Done

Confirm prerequisites passed, bootstrap completed, tests pass, and ask what the user wants to tackle first.
