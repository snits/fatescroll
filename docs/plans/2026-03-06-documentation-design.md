# Documentation Design

## Overview

User documentation for fatescroll, targeting two audiences: end users (GMs who want to roll tables) and table authors (people creating their own collections).

## Documents

### README.md

Project entry point. Covers installation, quick start, and CLI usage with one realistic example per subcommand.

**Sections:**

1. **Project overview** — What fatescroll is and what it does
2. **Installation** — Build from source with cargo
3. **Quick start** — Create a minimal table and roll on it
4. **CLI usage** — One example with sample output per subcommand:
   - `validate` — Check a collection for errors
   - `roll` — Roll on a table by FQID
   - `search` — Find tables by name, tag, or namespace
   - `import` — Add table files to a collection
5. **Documentation links** — Points to authoring guide
6. **License** — MIT

**Style:** Moderate detail. Show what the tool does without duplicating `--help`.

### docs/authoring-guide.md

Comprehensive guide for creating table collections.

**Sections:**

1. **Collections** — What a collection is, manifest.yaml schema, directory layout
2. **Namespaces and FQIDs** — How namespaces map to directories, how fully-qualified IDs work
3. **Table types** — Simple tables (dice expression, min/max ranges, text) and compound tables (roll multiple sub-tables)
4. **Chaining** — Results that trigger rolls on other tables, relative resolution within namespace
5. **Dice interpolation** — `{2d6x10}` syntax in result text for inline dice evaluation
6. **Validation rules** — What the validator checks (range gaps/overlaps, bad dice expressions, unresolved chain/compound references)
7. **Example collection** — Complete worked example with manifest and several interconnected tables
