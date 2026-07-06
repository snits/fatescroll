# Handoff: Table Forge — Fatescroll collection editor

## Overview
**Table Forge** is a desktop editor for authoring *Fatescroll* random-table collections (the kind used for tabletop RPG generators). A user builds a collection of dice tables organized into directories, edits each table's roll expression + weighted results, chains tables together, and exports the whole thing as a folder of `.yaml` files (or a `.zip`). A live pane on the right shows the emitted YAML, real-time validation, and a "cast the dice" test roller.

The interface is a classic three-pane IDE layout: **tree (left) · editor (center) · YAML + validation + roller (right)**, wrapped in a "medieval scriptorium" visual theme (aged parchment, oxblood/gold accents, blackletter display type).

## About the Design Files
The files in this bundle are a **design reference created in HTML** — a working prototype showing the intended look, layout, and behavior. They are **not production code to copy directly.** `Table Forge.dc.html` is authored as a "Design Component": its markup uses custom `<x-dc>`, `<sc-for>`, `<sc-if>`, and `{{ … }}` template holes, and its logic lives in a `class Component extends DCLogic`. That runtime (`support.js`) is a prototyping harness, not something to ship.

Your task is to **recreate this design in the target codebase's existing environment** (React, Vue, Svelte, SwiftUI, etc.) using its established patterns, state management, and component libraries. If no codebase exists yet, choose the most appropriate framework and implement it there. Treat the HTML as the spec for *what to build and how it should look/behave* — the state shape and algorithms in the logic class are directly portable; the template syntax is not.

## Fidelity
**High-fidelity.** Final colors, typography, spacing, interactions, and the full data model / validation / dice logic are all present and intended. Recreate the UI pixel-closely using the codebase's own primitives. The dice-probability, validation, YAML-emit, and zip-export algorithms below are the real intended behavior — port them faithfully.

---

## Layout (top level)
- Full-viewport flex column, `height: 100vh`.
- **Header bar** (fixed height, ~56px) across the top.
- **Body**: `flex: 1`, horizontal flex, three panes:
  - Left rail (tree) — `flex: 0 0 264px`
  - Center (editor) — `flex: 1`, scrollable
  - Right pane (YAML/validation/roller) — `flex: 0 0 440px`
- Page background: `#e3d7bb` with two faint radial-gradient overlays (top-left warm white `rgba(255,250,235,.5)`, bottom-right olive `rgba(160,130,80,.10)`).
- Base text: `#362d1e`, font `Spectral, Georgia, serif`, 15px.

---

## Screens / Views

There is one screen with three mutually-exclusive **center-pane states**, driven by `view` (`'empty' | 'manifest' | 'table'`) plus the selected table id.

### Header bar
- Background `#3a2f1c`, text `#eaddb9`, bottom border `3px double #9c7a2f`, padding `12px 22px`, gap 20px.
- **Brand block** (left): "Fatescroll" in `IM Fell English SC` 26px `#e9d9a8`; under it "TABLE FORGE" in `IM Fell English SC` 12px, letter-spacing 3px, `#b79a5f`.
- Vertical divider `1px × 34px`, `#6a5834`.
- **Collection label**: tiny uppercase "COLLECTION" label `#9c855c` 11px; below it the manifest name `#f0e6c8` 16px.
- Spacer (`flex:1`).
- **Status pill**: dot + text. Valid → green (dot `#8fbf6a`, border `#5f7a3a`, bg `rgba(120,160,80,.18)`, fg `#dbe7b8`, text "Collection is valid"). Invalid → amber (dot `#d98a5a`, border `#9c5a34`, bg `rgba(180,90,50,.18)`, fg `#eec6a6`, text "N error(s)"). Dot has a matching `box-shadow: 0 0 8px` glow.
- **Export button**: "Export collection ▾", padding `9px 16px`, border `1px solid #c79a3d`, radius 3px, bg `#9c7a2f` (hover `#b18a37`), fg `#fbf3d8`, 13px 600. Triggers the zip export.

### Left rail — "SCRIPTORIUM" (the tree)
- Background `#efe6cf`, right border `1px solid #c8b28a`, vertical flex.
- Header label "SCRIPTORIUM" — `IM Fell English SC` 13px, letter-spacing 2px, `#8a744c`.
- Scrollable body:
  - **manifest.yaml node**: row with ⚜ glyph + "manifest.yaml". Selected state (view==='manifest'): bg `#e0d3b0`, border `#8b2b2b`, fg `#5c1f1f`, weight 600. Idle: transparent bg, border `#d2c199`, fg `#5c4a2c`, weight 400. Hover border `#a98f63`.
  - **Per directory**: header row shows `path/` (13px 600 `#5c4a2c`) with the namespace under it in `JetBrains Mono` 10px `#a08a5f`. A `+` button (22×22, border `#c0a76c`, bg `#f7f0dc`, hover `#eadfbf`) adds a new table to that directory. Clicking the dir header opens the manifest view.
  - **Per table** (indented 12px, left-accent border): a type badge — `smp` (bg `#e7ead6`, fg `#4f5d38`) or `cmp` (bg `#e2d3ea`, fg `#6a4a86`), `JetBrains Mono` 9px — then the table name (13px, ellipsized). Selected: bg `#e0d3b0`, left-accent `#8b2b2b`, fg `#5c1f1f`, weight 600. Idle: transparent, accent transparent, fg `#5c4a2c`. Hover bg `#e7dcbe`.
  - **"+ add directory"** dashed button at the bottom (border `1px dashed #b39a68`, `#8a744c`, hover bg `#e7dcbe`).

### Center — Empty state
Shown when nothing is selected. Centered column: ✦ glyph (40px, opacity .5), "Nothing selected" (`IM Fell English SC` 20px), and helper line (14px, max-width 320px). All `#8a744c`.

### Center — Manifest editor
`max-width: 640px`.
- Title "Collection Manifest" (`IM Fell English SC` 22px `#4a3a1e`) + subtitle referencing `manifest.yaml`.
- **2-column grid of fields** (gap `16px 20px`): Name, Version (mono), Root namespace (mono, red border `#b05a5a` if invalid), Author (optional, placeholder `~`), Min tool version (optional, mono, placeholder `~`).
  - Field label: 12px uppercase letter-spacing 1px `#8a744c`; optional suffix in normal-case `#b0996b`.
  - Input: padding `8px 10px`, border `1px solid #c8b28a`, radius 3px, bg `#fbf6e9`, 14px, focus border `#8b2b2b`.
- **DIRECTORIES section**: header (`IM Fell English SC` 15px letter-spacing 1.5px `#7a6033`) with bottom border `#c8b28a` and an "+ add" button. Each directory is a `1fr 1fr auto` grid row: `path` input, `namespace` input (red border if invalid), and an `✕` delete button (34×34, red `#9a3030`, hover bg `#f0dede`).

### Center — Table editor
`max-width: 720px`.
- **Header row**: "Display name" input (large — 19px `IM Fell English SC`) + a "Delete" button (red).
- **Row**: "File / id" input (mono; suffix note "— becomes `<stem>.yaml`") + a **Type segmented control** (two buttons `simple` | `compound`; active button bg `#8b2b2b` fg `#f4e3c8`, inactive bg `#f2ead6` fg `#7a6033`).
- **FQID line** (mono 11px): `FQID · <namespace>.<stem>`.
- **Tags** input (comma-separated, stored as array).

#### Simple table body
- **Roll** input (mono, 130px wide; red border if dice unparseable) with an info line beside it:
  - valid normal dice → `range <min>–<max> · N outcome(s)` (`#4f5d38`)
  - `d66` → `D66 · 36 outcomes (11–66)`
  - invalid → `unparseable dice expression` (`#a11f1f`)
- **modifier_range** row: a checkbox `modifier_range`; when on, two number inputs `[ min , max ]` (mono) and an "effective span `<lo>–<hi>`" note.
- **RESULTS section** header with two buttons:
  - **⚖ Auto-fill ranges** (olive: border `#6f7d4f`, bg `#eef0e0`, fg `#4f5d38`) — distributes ranges evenly with no gaps across the effective dice span.
  - **+ result** (border `#c0a76c`, bg `#f7f0dc`).
- **Each result card**: border `1px solid #cdb88f` with left accent `3px solid #9c7a2f`, bg `#f6efdc`, radius 3px, padding `12px 14px`.
  - Top row: `min` – `max` number inputs (mono, 46px each), a **probability pill** (mono 11px, bg `#e7ead6`, border `#c3cba3`, fg `#4f5d38`, computed from the dice distribution), spacer, and `✕` remove.
  - **Text** input full-width (placeholder mentions `{2d6x10}` dice interpolation).
  - **Chain** block (indented, left border `2px dotted #c3a86e`): each chain row = `↳` glyph + a `table reference` input (red border if unresolved) + an optional `reroll` field (only when the chain entry is "structured", toggled by the `↺` button; active toggle bg `#8b2b2b`) + `↺` toggle + `✕` remove. "+ chain to table" dashed button adds one.
- **Notes** textarea (one note per line).

#### Compound table body
- **SUB-TABLES section** header + "+ table" button.
- Explanatory line: each sub-table is rolled when the compound table is rolled; results combined.
- Each row: `◈` glyph + `table reference` input (red border if unresolved) + `✕` remove.

### Right pane — YAML / Validation / Roller
Background `#efe6cf`, left border `1px solid #c8b28a`, vertical flex.
- **Header**: title (`MANIFEST.YAML`, `<STEM>.YAML`, or `YAML`) + **copy** button (label toggles `⧉ copy` → `✓ copied` for 1.4s) + **⬇ .yaml** download button.
- **YAML viewer**: dark panel — bg `#241d12`, border `1px solid #2f2718`, radius 4px. `<pre>` in `JetBrains Mono` 12.5px, line-height 1.65, color `#d9c99a`, `white-space: pre`, scrollable. Content is the live-emitted YAML for the current view.
- **VALIDATION** section (max-height 34%, scrollable): label + mono subtitle "fatescroll validate". If valid → green `✓ Collection is valid.` (`#4f5d38`). Otherwise a list of problems, each mono 11.5px: `✕` + message in `#a11f1f` (errors) or `!` + `#9c6a2f` (warnings).
- **CAST THE DICE** section: label + a **⚄ Roll** button (oxblood: border `#7a2323`, bg `#8b2b2b` hover `#a03434`, fg `#f4e3c8`). Below, an output panel (bg `#f6efdc`, border `#cdb88f`) showing either the placeholder "— roll a table to preview an outcome —" or indented result lines. Each line is indented `indent * 18px`; depth-0 lines `#3a2f1c`, deeper `#6b5535`, errors `#a11f1f`.

---

## Data model (state shape)

```
manifest: { name, version, namespace, author, minToolVersion }
dirs:     [ { id, path, namespace } ]
tables:   [ Table ]
view:     'empty' | 'manifest' | 'table'
selUid:   <uid of selected table | null>

Table = {
  uid, dirId, stem, name,
  type: 'simple' | 'compound',
  tags: string[],
  roll: string,                 // dice expression, simple only
  modOn: bool, modMin, modMax,  // modifier_range, simple only
  notes: string[],              // simple only
  results: Result[],            // simple only
  tablesRefs: [ { rid, ref } ], // compound only
}
Result = {
  rid, min: string, max: string, text: string,
  chain: [ { rid, struct: bool, ref: string, table: string, reroll: number[] } ]
}
```
- **Directory ↔ namespace**: every table lives in a directory; the table's namespace is that directory's `namespace`. **FQID** = `<dir.namespace>.<stem>`.
- Numeric fields (`min`/`max`/`modMin`/`modMax`) are stored as **strings** (raw input) and parsed on use.

---

## Core algorithms (port these faithfully)

### Dice engine
Two functions over a dice expression string:
- **`diceInfo(expr)`** → full probability distribution. Supports:
  - `d66` / `66` → special: 36 outcomes, values `11..66` where each is `tens*10 + units` (tens & units each 1–6), uniform `1/36`. min 11, max 66.
  - Standard `NdF` grammar via regex `^(\d*)d(f|\d+)(?:\s*[x*]\s*(\d+))?\s*([+-]\s*\d+)?$`:
    - `N` dice count (1–40, default 1), faces = `d<sides>` (sides 1–1000) or `dF` = Fudge dice with faces `[-1,0,1]`.
    - Optional `x<mult>` (or `*`) multiplier and optional `+K` / `-K` additive modifier.
    - Builds the exact convolution distribution (map of value→probability), returns sorted `values`, `dist`, `min`, `max`, `kind`.
  - Returns `{ ok: false }` on anything unparseable / out of bounds.
- **`sampleRoll(expr)`** → a single random outcome using the same grammar (used by the roller).

### Probability display
For each result row, sum `dist` probabilities for all values in `[min,max]`. Format: `0%` if zero; one decimal if `<10%`, else rounded integer + `%`. `—` if unparseable.

### Text interpolation
Result text may embed dice like `{2d6x10}` — replace each `{expr}` with a fresh `sampleRoll(expr)` when rolling. Unparseable braces are left as-is.

### Auto-fill ranges
Computes the effective contiguous span `[lo,hi]` (dice min/max, shifted by modifier_range if on). For `d66`, emits one row per value (preserving existing text/chain by index). Otherwise splits the span into `k = number of existing results` contiguous sub-ranges as evenly as possible (larger chunks first), preserving each row's existing text/chain by index. Result: full coverage, no gaps, no overlaps.

### Validation (`fatescroll validate`)
Produces an ordered list of errors:
- **Namespace check** on root manifest namespace and every directory namespace: each dot-segment must match `^[a-z][a-z0-9_-]*$`.
- **Per table**:
  - *compound*: every `tablesRefs[].ref` must resolve, else `unresolved compound table reference '<ref>' in table '<name>'`.
  - *simple*: dice must parse (`invalid dice expression …`). Then check results:
    - reversed range (`min > max`) → `range reversed …`.
    - Compute **expected values** = all dice values (each ⊕ every modifier offset if modifier_range on). Build a coverage count over `[min,max]` of every result. `missing` expected values → `range gap … missing values [ … ]`. Values covered more than once → `range overlap … values [ … ] covered multiple times`.
  - Every chain `ref` must resolve, else `unresolved chain reference …`.
- **Reference resolution** (`resolve(ref, fromNs, map)`): try `<fromNs>.<ref>` first (relative), then `<ref>` as an absolute FQID. `map` = FQID→table.

### Test roller (`doRoll` / `rollTable`)
Recursively rolls the current table, appending indented lines:
- Depth limit 10 (guards cyclic chains) → error line.
- *compound*: print table name, then roll each resolved sub-table at indent+1; unresolved → `· unresolved: <ref>` error.
- *simple*: `sampleRoll` (re-rolling while the value is in an active `reroll` set, up to 100 tries); find the matching result row; interpolate its text; print `<name> (rolled <v>): <text>`. Then for each chain entry, roll the resolved child at indent+1 (passing the chain's `reroll` set if it's a structured entry).

### YAML emit
- **`manifestYaml()`**: `name`, `version` (always double-quoted), `namespace`, `author` (or `~`), `min_tool_version` (or `~`), then a `directories:` list of `{ path, namespace }`.
- **`tableYaml(t)`**: `id` (=stem), `name`, `type`, optional `tags` list. Compound → `tables:` list of refs. Simple → `roll`, optional `modifier_range: [min, max]`, optional `notes` list, then `results:` — each with `min`, `max`, optional `text`, and optional `chain:` (plain string entries, or `{ table, reroll: [...] }` for structured entries).
- **`yv(s)` scalar quoting**: quote a YAML scalar when empty, has leading/trailing space, starts with a YAML indicator char, contains braces/brackets, contains `: ` or ` #`, is a boolean/null-ish keyword, or looks numeric. Escape `\` and `"` inside quotes.

### Export
- **Copy / download current**: copies or downloads the YAML for the active view (`manifest.yaml` or `<stem>.yaml`).
- **Export collection (zip)**: builds a store-method (uncompressed) ZIP in-browser — no library. Structure:
  ```
  <collection-slug>/
    manifest.yaml
    <dir.path>/<stem>.yaml   (one per table, grouped by its directory)
  ```
  Slug = collection name lowercased, non-alphanumerics → `-`. Includes a hand-rolled CRC32 + ZIP local/central-directory writer (see `makeZip` / `crc32` in the logic class — port or replace with the target platform's zip library).

---

## Interactions & Behavior
- Selecting the manifest node, a directory header, or a table swaps the center editor and the right-pane YAML/validation target.
- Editing any table field re-derives YAML, validation, and status live; it also **clears the last roll result** (`rollLines: null`).
- `+` buttons add directories (→ opens manifest view), tables (→ selects the new table), results, chains, and sub-table refs.
- Deleting a directory with tables asks for confirmation and removes its tables too. Deleting a table asks for confirmation and selects the next remaining table (or empty state).
- Copy button label animates `⧉ copy` → `✓ copied` (1.4s) via clipboard API.
- `stem` input strips whitespace to `-`; tag input splits on commas; number inputs strip non-numeric chars.
- No hover animation beyond simple color/border swaps (see per-component states above). No transitions specified.

---

## Design Tokens

**Fonts** (Google Fonts): `IM Fell English SC` (display / blackletter-flavored headings), `Spectral` (body serif, weights 300–700 + italic), `JetBrains Mono` (code/YAML/numeric, 400–600).

**Colors**
- Parchment bg: `#e3d7bb`; panel bg: `#efe6cf`; card bg: `#f6efdc`; input bg: `#fbf6e9`; button bg (neutral): `#f7f0dc` / `#f2ead6`.
- Dark header/wood: `#3a2f1c`; header text `#eaddb9`; brand gold `#e9d9a8` / `#b79a5f`.
- Text: primary `#362d1e`, secondary `#5c4a2c`, muted `#8a744c`, faint `#a08a5f` / `#b0996b`.
- Borders: `#c8b28a` (inputs), `#cdb88f` / `#cbb08a`, `#c0a76c` (buttons), `#d2c199` (idle tree).
- **Oxblood accent** (primary action / selection / focus): `#8b2b2b` (hover `#a03434`), border `#7a2323`, selection text `#5c1f1f`, active bg `#e0d3b0`.
- **Gold accent** (export / result left-border): `#9c7a2f` (hover `#b18a37`), border `#c79a3d` / `#c797... ` , text `#fbf3d8`.
- **Olive** (auto-fill / valid / probability): `#4f5d38`, border `#6f7d4f`, bg `#eef0e0` / `#e7ead6`.
- Error red: `#a11f1f` / `#9a3030`, red border `#b05a5a`, error bg `#f0dede`.
- Compound badge: bg `#e2d3ea`, fg `#6a4a86`.
- YAML panel: bg `#241d12`, border `#2f2718`, text `#d9c99a`.
- Selection highlight (`::selection`): bg `#d8c08a`, text `#2a2314`.

**Radii**: 3px (nearly everything), 4px (YAML panel), 50% (status dot), 10px (probability pill).

**Borders of note**: header bottom `3px double #9c7a2f`; result card left `3px solid #9c7a2f`; chain block left `2px dotted #c3a86e`.

**Scrollbars**: 11px, thumb `#c1a978` with a 3px transparent content-box border (inset pill look), transparent track.

**Spacing**: pane widths 264 / flex / 440px; center padding `22px 26px`; typical field gap 5px label→input, 16–20px between field groups.

---

## Assets
No image assets. Icons are Unicode glyphs: ⚜ ✦ ⚖ ↳ ↺ ◈ ✕ ⚄ ⬇ ⧉ ✓ ▾ ⚜. Replace with the codebase's icon set if preferred, or keep as text glyphs.

## Files
- `Table Forge.dc.html` — the complete design + logic (template markup in the `<x-dc>` block; state, dice engine, validation, YAML emit, zip export in the `class Component` script).
- `support.js` — the prototyping runtime that renders the Design Component (reference only; **do not ship**).

To view the prototype as-authored, open `Table Forge.dc.html` in a browser. To implement, read the `class Component` logic for the exact algorithms and the `<x-dc>` template for the exact markup/styles, and rebuild in your target framework.
