# Optional `notes` field on tables with `--notes` show flag

Bead: fatescroll-6ff (split from fatescroll-0ax)

## Problem

Some tables determine their result through a procedure that isn't obvious from the
roll expression alone — opposed rolls, modifier sources, or complex DM-supplied
adjustments. There is currently no place to record a short reminder of that
procedure on the table itself. A GM reading `fatescroll show` has to remember the
convention or keep the rulebook open.

## Goal

Add an optional `notes` field (list of strings) to tables for a quick reminder of
how the value is determined. Surface it through `fatescroll show --notes`. Default
`show` output stays clean; the roller ignores notes entirely.

Not a rulebook replacement — just a couple of lines of context.

## Design

### Data model (`models.rs`)

Add `notes: Vec<String>` to both `Table::Simple` and `Table::Compound`, mirroring
the existing `tags` field exactly:

```rust
#[serde(default)]
notes: Vec<String>,
```

Add a `notes()` accessor alongside the existing `id()` / `name()` / `tags()`
accessors that spans both variants.

Rationale for both variants: the existing accessor pattern (`id`, `name`, `tags`)
already spans both `Simple` and `Compound`. A compound table can legitimately carry
a note (e.g. "combine occupation + disposition into one NPC line"). Following the
established pattern is more consistent than a Simple-only field and costs one extra
field plus one accessor arm.

Serde: match `tags` — `#[serde(default)]`, no `skip_serializing_if`. Verified that
nothing round-trips a `Table` struct back to YAML: `fixer.rs` manipulates a
`serde_yaml::Value` mapping directly and `init.rs` builds YAML from a string
template, so adding the field will not cause `notes: []` to be written into any
on-disk table file.

### Display (`display.rs`)

Add a `show_notes: bool` parameter to `format_table`. Notes rendering belongs in
`display` next to `modifier_range`, not bolted onto the CLI. When `show_notes` is
true and the table has notes, render a `Notes:` block after the header lines
(name / tags / Roll / Modifier) and before the results grid / sub-table list:

```
Carousing (ns.carousing)
Roll: 1d8
Modifier: 0 to 6
Notes:
  - Attacker rolls 2d6 minus defender 2d6
  - DMs: +2 boarding equipment, -1 per 1000 tons difference

  1   ...
```

When `show_notes` is false, or the table has no notes, no `Notes:` line appears —
output is byte-for-byte what it is today.

### CLI (`fatescroll-cli/src/main.rs`)

Add a `--notes` boolean flag to the `Show` subcommand. Thread it through `cmd_show`
into `format_table(table_id, table, show_notes)`.

### Out of scope (YAGNI)

- `init` template generation is left untouched — it does not emit `tags` or
  `modifier_range` placeholders either, and the bead does not ask for it.
- The roller is not modified; it already never reads `notes`. A test will assert
  this rather than any code change.
- `search` is not modified; notes are not searchable.

## Testing

- `models.rs`: deserialize a table with `notes`; deserialize one without (defaults
  to empty); JSON/YAML round-trip with notes present.
- `display.rs`: `format_table(.., true)` on a table with notes renders the `Notes:`
  block and each note; `format_table(.., false)` omits it; a table without notes
  shows nothing even with `show_notes = true`; notes block sits before the results.
- `roller`: load a fixture table carrying notes, roll it, assert the result is
  unaffected (notes ignored).
- CLI integration (`cli_integration.rs`): `show <table> --notes` prints notes;
  `show <table>` without the flag does not.
- Fixture: add a `notes` field to one table in `tests/fixtures/valid-collection/`
  (or a dedicated fixture) so integration tests have real data.

## Approach alternatives considered

1. **`show_notes` param on `format_table` (chosen)** — keeps all table rendering in
   `display`, single caller threads one bool.
2. Separate `format_notes(table) -> Option<String>` called by the CLI — splits
   table rendering across two layers; rejected.
3. Notes on `Simple` only — narrower, but breaks the both-variants accessor pattern
   for no real benefit; rejected.
