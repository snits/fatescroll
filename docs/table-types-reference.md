# TTRPG Random Table Types Reference

A catalog of mechanically distinct random table types found across tabletop RPGs,
organized by whether fatescroll should support them natively, via modifiers, or
leave them to application-level tooling.

## Supported by fatescroll

### Simple Lookup (XdY → entry)
Roll dice, sum result, look up entry by range. The core table type.
- **Examples**: Most D&D random tables, OSR encounter tables
- **fatescroll**: `type: simple` with `roll` and `results`

### Compound / Multi-Table (roll on several sub-tables)
A single table that rolls on multiple sub-tables simultaneously, combining results.
- **Examples**: NPC generators (roll appearance + personality + quirk)
- **fatescroll**: `type: compound` with `sub_tables`

### Chained / Nested ("Matryoshka")
Rolling on Table A produces a result that triggers a roll on Table B.
- **Examples**: Encounter type → specific encounter sub-table
- **fatescroll**: Chain references in result entries

### Tiered / Bucketed (PbtA style)
The probability space is grouped into broad ranges rather than individual entries.
- **Examples**: PbtA moves (6-, 7-9, 10+), reaction rolls
- **fatescroll**: Range entries with `min`/`max` spanning multiple values

### Keyword / Prompt Tables
Entries are evocative keywords rather than detailed descriptions. The user
synthesizes meaning from the result.
- **Examples**: Ironsworn Action/Theme oracles, solo play word tables
- **fatescroll**: Just text entries — already works

### Bell Curve / Boolean (Fudge/Fate)
Centered distribution where the middle result dominates. Uses dice that
produce distributions clustered around zero or a midpoint.
- **Examples**: Fate/Fudge (4dF, range -4 to +4), weather tables
- **fatescroll**: Supported if diceman handles the dice type (dF supported)

### D66 Tables (Traveller style)
Roll 2d6, read as tens and units (11-16, 21-26, ... 61-66). Produces 36
non-contiguous values.
- **Examples**: Traveller world generation, many Troika tables
- **fatescroll**: Planned (fatescroll-tr7). Requires non-contiguous range support.

## Supported via --modifier (planned: fatescroll-0ax)

### Static Modifier ("Buff/Debuff")
Modifier comes from a character stat or situational bonus. Temporary, localized
to one roll.
- **Examples**: Shadowdark carousing (1d8 + spending bonus 0-6, 14 entries)

### Escalating / State-Based Modifier
A persistent world-state variable affects every roll. The application tracks
the state and supplies the modifier.
- **Examples**: "Heat" level in heist games, escalating danger

### Negative Modifier
Subtractive modifiers that push results below the base dice minimum.
- **Examples**: Traveller aging (1d6 - terms completed, entries go negative)

### Feedback / "Death Spiral" Modifier
The table result instructs future rolls to use a modifier. The user or
application tracks the accumulated modifier.
- **Examples**: "Add +1 to all future Stability rolls"

### Difference / Margin of Success
The lookup index is the margin between roll and target, not the raw roll.
The application computes the margin and supplies it as a modifier.
- **Examples**: Pendragon opposed rolls, Cyberpunk skill checks

### Dynamic Range / Shifting Window
Entry ranges stay the same but difficulty shifts the effective roll. Equivalent
to applying a modifier.
- **Examples**: The One Ring difficulty adjustments

**Key insight**: All modifier variants are `--modifier N` from fatescroll's
perspective. The human or application computes the value; fatescroll clamps
the result to the table's entry range.

## Application-Level Concerns (not fatescroll)

These table types require state tracking, non-sum dice interpretation, or
fundamentally different lookup mechanics. They belong in game engines or
orchestrator tools that call fatescroll.

### Deck / Depletion / Check-off
Once a result is rolled, it is removed or replaced. Requires persistent state
between rolls.
- **Examples**: Mothership hazard tables, "sampling without replacement"

### Subtractive Tables
A list where consumed entries shift probability toward remaining entries.
Results move toward a specific direction as entries are depleted.
- **Examples**: Trophy Gold resource scavenging, Darkest Dungeon style

### Pool / Success Counting
The lookup index is the count of successes in a dice pool, not a sum.
- **Examples**: Vampire/Shadowrun (roll Xd10, count results >= threshold)

### Sum-of-Matches (ORE)
Multi-dimensional lookup: width (how many match) determines one axis,
height (face value) determines another.
- **Examples**: One Roll Engine hit location tables

### Over/Under (Sweet Spot)
The best result is rolling closest to a variable (your skill), not highest.
Fundamentally different lookup logic.
- **Examples**: Pendragon skill rolls

### Roll All (Cluster)
A pool of dice where each die is looked up individually against a small table,
generating multiple simultaneous events.
- **Examples**: Stygian Library depth crawl tables

### Matrix / Joint-Probability
Two independent rolls cross-referenced on a grid. One die selects the row,
another selects the column.
- **Examples**: Ironsworn Action + Theme oracles (when used as a pair)

### Accumulator / Clock
Rolls contribute to a counter rather than producing discrete results.
The actual event triggers when the counter reaches a threshold.
- **Examples**: Blades in the Dark progress clocks, travel encounter pools

### Recursive / Self-Modifying
The table result modifies the table itself for future rolls. Requires
mutable table state.
- **Examples**: "Encounter a Scout. Add +2 to all future rolls on this table."

## Design Principle

fatescroll's job: **given a number, look up a result in a table, optionally
chain to other tables.** Everything that computes *what number to look up*
(success counting, margins, ORE sets) or *tracks state between lookups*
(decks, depletion, clocks) belongs in the calling application. The `--modifier`
flag is the clean interface between fatescroll and stateful game logic.
