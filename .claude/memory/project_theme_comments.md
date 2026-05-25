---
name: Theme comment style
description: Theme struct comments should describe what is being themed, not prescribe color expectations
type: project
---

Theme field comments should describe the UI element being themed, not any expectation of color (e.g., "brighter than X" or "darker than Y"). The theme consumer decides the actual color; comments just identify the target.

**Why:** Colors are a themeable choice — embedding color expectations in comments couples documentation to a single theme's conventions and creates misleading guidance for future themes.

**How to apply:** When adding or editing `Theme` struct fields, write comments like "Active diff block indicator in minimap" rather than "Brighter version of minimap_diff". Also clean up existing comments that violate this when touching nearby code.
