# Show Command Design

## Overview

Add `fatescroll show <table_id>` to display a formatted view of a table's contents without rolling on it. Useful for authors previewing tables and players browsing available content.

Covers bead fatescroll-atw.

## Output Format

### Simple Tables

```
Wilderness Encounter (test.encounters.wilderness-encounter)
Tags: encounter, wilderness
Roll: 1d8

  1-3  Animal encounter → animal-type
  4-5  Bandit camp → bandit-strength, bandit-motivation
  6-7  Abandoned campsite
  8    Merchant with {2d6x10} gold → merchant-goods
```

**Formatting rules:**
- Header: table name, FQID in parentheses
- Tags line (omitted if no tags)
- Roll line showing the dice expression
- Results as a formatted table: range column, text, chain references with `→`
- Range collapse: when min == max, show single value (e.g., `8` not `8-8`)
- Range column right-aligned for visual consistency

### Compound Tables

```
Quick NPC Generator (test.npc.quick-npc)
Tags: npc, generator
Tables:
  - npc-occupation
  - npc-disposition
  - npc-quirk
```

## Implementation

- Add `format_table` function in a new `src/display.rs` module
- Takes `(&str, &Table)` (FQID and table reference), returns `String`
- `cmd_show` in main.rs: resolve collection, load registry, resolve table, call `format_table`, print
- Unit tests in display.rs with known tables, integration test via CLI

## Scope

- Display only — no rolling, no interactive features
- No color/styling (keep it simple, pipeable)
- No recursive display of chained tables (just show the reference names)
