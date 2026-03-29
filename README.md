# fatescroll

A structured randomness engine for creative and evaluative work.

## Overview

fatescroll loads collections of random tables defined in YAML, validates them,
and rolls on them from the command line. Tables support chaining (a result
triggers rolls on other tables), compound tables (roll multiple tables at once),
and inline dice interpolation in result text (e.g., `{2d6x10}` evaluates to a
rolled value).

## Installation

Build from source (requires Rust):

```
git clone https://github.com/snits/fatescroll.git
cd fatescroll
cargo install --path .
```

## Quick Start

Create a minimal collection to try out fatescroll.

### 1. Set up the collection directory

```
mkdir -p my-collection/tavern
```

### 2. Create `my-collection/manifest.yaml`

```yaml
name: My Collection
version: "1.0"
namespace: my
directories:
  - path: tavern
    namespace: my.tavern
```

### 3. Create `my-collection/tavern/tavern-events.yaml`

```yaml
name: Tavern Events
type: simple
tags:
  - tavern
  - social
roll: 1d6
results:
  - min: 1
    max: 2
    text: A bard starts an off-key ballad
  - min: 3
    max: 4
    text: Two drunken patrons start a fistfight
  - min: 5
    max: 5
    text: A hooded stranger beckons from a corner booth
  - min: 6
    max: 6
    text: "The barkeep offers a round on the house worth {2d6} silver"
```

### 4. Validate the collection

```
$ fatescroll validate my-collection
Collection is valid.
```

### 5. Roll on the table

```
$ fatescroll roll --collection my-collection my.tavern.tavern-events
Tavern Events (rolled 3): Two drunken patrons start a fistfight
```

## CLI Usage

### validate

Validate a collection directory:

```
$ fatescroll validate path/to/collection
Collection is valid.
```

### roll

Roll on a table by its fully qualified ID:

```
$ fatescroll roll --collection path/to/collection test.encounters.wilderness-encounter
Wilderness Encounter (rolled 4): Bandit camp
  Bandit Strength (rolled 1): Small group (2-3 bandits)
  Bandit Motivation (rolled 2): Organized toll collectors
```

Chained results are indented beneath the parent roll.

### search

Find tables by name, tag, or namespace:

```
$ fatescroll search --collection path/to/collection --tag encounter
  test.encounters.wilderness-encounter — Wilderness Encounter [encounter, wilderness]
  test.encounters.bandit-motivation — Bandit Motivation [encounter, bandit]
  test.encounters.merchant-goods — Merchant Goods [encounter, merchant]
  test.encounters.bandit-strength — Bandit Strength [encounter, bandit]
  test.encounters.animal-type — Animal Type [encounter, animal]
```

You can also search with `--name` or `--namespace` instead of `--tag`.

### import

Import table files into a collection:

```
$ fatescroll import --collection path/to/collection --target-dir monsters goblin.yaml troll.yaml
Imported: goblin.yaml
Imported: troll.yaml
Validating collection...
Collection is valid after import.
```

## Documentation

See [docs/authoring-guide.md](docs/authoring-guide.md) for details on writing
tables, chaining, compound tables, and dice expressions.

## License

MIT -- see [LICENSE](LICENSE).
