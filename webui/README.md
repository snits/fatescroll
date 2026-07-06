# Table Forge

Browser editor for authoring fatescroll YAML collections — build tables, chains,
and compound tables in a form UI, preview the emitted YAML, validate and roll
against the real fatescroll engine, and export a zip ready to drop into a
collection directory.

## Prerequisites

- Node.js (developed against v22)
- Rust toolchain + [`wasm-pack`](https://rustwasm.github.io/wasm-pack/) (`cargo install wasm-pack`) — builds `fatescroll-wasm` to WebAssembly

## Setup

```bash
cd webui
npm install
```

## Development

```bash
npm run dev          # build:wasm, then start the Vite dev server
```

## Testing

```bash
npm test             # vitest: unit, component, and golden round-trip tests
npm run test:watch   # vitest in watch mode
```

`tests/golden-roundtrip.test.ts` compiles `fatescroll-cli` in debug mode and
drives the real binary, so the first test run is slow (workspace build) —
subsequent runs are incremental.

## Build

```bash
npm run build        # build:wasm, then tsc -b, then vite build
npm run lint          # oxlint
```

## Architecture

```
store (Zustand)  ->  YAML emitter  ->  fatescroll-wasm engine  ->  right pane
```

- **`src/model/store.ts`** — Zustand store holding the manifest, directories,
  and table drafts, plus UI selection state (`view`/`selUid`) and the last
  dice-roll preview.
- **`src/yaml/emit.ts`** — turns store state into fatescroll YAML text. Output
  must parse byte-reliably in the real Rust CLI's `serde_yaml` loader.
- **`fatescroll-wasm`** (`../fatescroll-wasm`) — wraps `fatescroll-core` with
  `wasm-bindgen`, exposing five JSON-string functions: `validate_collection`,
  `dice_info`, `expected_values`, `histogram`, `roll_collection`. Built by
  `npm run build:wasm` into `src/wasm/pkg/` (gitignored, regenerated on every
  `dev`/`build`).
- **`src/engine/engine.ts`** — wraps the raw wasm module: parses its JSON
  envelopes, memoizes dice queries per call, and generates a fresh RNG seed
  for each roll.
- **`src/components/RightPane.tsx`** — renders `YamlViewer`, `ValidationPanel`,
  and `DiceRoller` from the current store state, calling the engine for
  validation and rolling.
- **`src/export/zip.ts`** — packs the manifest and table YAML into a zip
  archive (via `fflate`) mirroring the on-disk layout fatescroll expects.

The design premise worth remembering: validation and rolling in the browser
run the *same* `fatescroll-core` code as the CLI, so there is a single source
of truth for table semantics. `tests/golden-roundtrip.test.ts` proves this
end to end — it emits YAML from editor state, writes it to disk, and drives
the real compiled `fatescroll` binary to validate and roll it.
