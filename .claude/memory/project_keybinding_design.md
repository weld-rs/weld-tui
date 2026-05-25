---
name: Keybinding design philosophy
description: No command mode — single-key bindings with vim-style chords; behavior adapts to state rather than disabling bindings
type: project
---

Single-key bindings with chords (`q!`, `gg`), no command mode (`:w`, `:q`).

**Why:** Command mode solves keymap crowding that doesn't exist in weld. Weld has one mode and a small keymap. Reversible — can add command mode later if future commands (`:w <path>`, `:e`, search) accumulate enough to justify it.

**How to apply:**
- New quit/save/write features use single-key or chord bindings, not `:` prefixed commands
- Chords use `pending_*` fields in `InputState` with `CHORD_TIMEOUT` (500ms, configurable: #38)
- `q` behavior adapts to state (clean → quit, dirty → start `q!` chord) — don't disable bindings
- `w` adapts to state: one-side dirty → save silently, both-dirty → SavePicker overlay (l/r/a/Esc)
- `wq` works by composition (`w` saves, then `q` quits clean) — no dedicated chord
- Avoid chord mnemonics that conflict with vim motions (e.g., `wl`/`wr` rejected — `l` is ambiguous)
- Status bar hints update contextually to show what's available
- #29 closed — superseded by single-key bindings in #27
