# Result expressions — design proposal

Issue: `kmgh` · Date: 2026-09-04 · Baseline: `e522c6863a99`

Status: Proposed for Jerry's review. Jerry confirmed computed values and
conditionals in a small expression language, with values local to one selected
result. The syntax and remaining semantics below are recommendations, not yet
approved implementation requirements.
Jerry also explicitly approved a one-time migration preserving saved Table Forge
drafts and initializing existing results with an empty values list.

## Purpose and acceptance

The reference is FateScroll's existing inline dice in RPG table results, extended
with the few formulas a table author would otherwise work out by hand. The target
is a personal table-authoring tool with precise, predictable evaluation rules.

An author can roll a quantity once, derive another value from it, and choose
wording based on either value. The same table works in CLI and Table Forge;
opening, editing, and exporting it preserves its expressions. Invalid expressions
produce actionable diagnostics, and values never cross result or table boundaries.

## Alternatives

| Approach | Benefit | Cost | Decision |
| --- | --- | --- | --- |
| Ordered named expressions plus explicit text interpolation | Compact YAML; preserves the result/text/chain model | Small parser and evaluator to maintain | Recommended |
| Structured YAML expression trees (`if`, `multiply`, operands) | No textual expression precedence to learn | Verbose for ordinary formulas; larger editing surface | Reject for this scope |
| Embedded general scripting engine | Many operators and facilities supplied | Much broader language, runtime policy, and dependencies than these use cases need | Reject for this scope |

## Authoring contract

```yaml
id: gems
name: Gems
type: simple
roll: 1d6
results:
  - min: 1
    max: 6
    let:
      - name: count
        value: 'roll("1d4")'
      - name: price
        value: 'count * 25'
    text: >-
      Found {= count} {= if count == 1 then "gem" else "gems"}
      worth {= price} gold.
```

`let` is an optional ordered sequence of `{name, value}` objects. `value` is
always an expression string, even for a numeric constant. A sequence makes
dependencies and dice order explicit without depending on YAML map ordering.
Absent `let` means an empty sequence; an explicitly empty sequence is also valid.
Null, duplicate object keys, unknown binding fields, and non-string values are
rejected in the binding structure. Names must be unique within the sequence.

The built-in integer `value` is the effective number used to select this result:
the clamped lookup for a modified roll, the explicit number for `--value`, or the
digit value for D66. It is not an additional dice roll. Binding names cannot
shadow `value`, `roll`, `if`, `then`, `else`, `true`, or `false`.

Each binding may refer to `value` and earlier bindings only. Self-reference and
forward references fail validation. The template sees all bindings. Result
invocations have fresh environments, including repeated visits through chains;
there is no parent, child, sibling, global, or persistent variable access.

## Grammar

```ebnf
expression  = conditional ;
conditional = "if", expression, "then", expression, "else", expression | disjunction ;
disjunction = conjunction, { "||", conjunction } ;
conjunction = equality, { "&&", equality } ;
equality    = comparison, [ ("==" | "!="), comparison ] ;
comparison  = sum, [ ("<" | "<=" | ">" | ">="), sum ] ;
sum         = product, { ("+" | "-"), product } ;
product     = unary, { ("*" | "/" | "%"), unary } ;
unary       = ("-" | "!"), unary | primary ;
primary     = integer | boolean | string | identifier
            | "(", expression, ")" | "roll", "(", string, ")" ;
boolean     = "true" | "false" ;
integer     = digit, { digit } ;
identifier  = (letter | "_"), { letter | digit | "_" } ;
```

Identifiers use ASCII letters and are case-sensitive. Keywords are whole tokens.
Whitespace is insignificant between tokens. Consume the entire expression;
trailing input is an error. Conditional expressions used as operands require
parentheses. Arithmetic operators of equal precedence associate left to right.
Repeated comparisons or equalities at the same grammar level are rejected:
`value < 3 < 5` and `value == 3 == true` fail. Mixed precedence follows the EBNF:
`value < 3 == true` means `(value < 3) == true` and is accepted. Parenthesized
forms are preferable when writing mixed comparisons for clarity.

Strings are double-quoted UTF-8 text. Supported escapes are `\"`, `\\`, `\n`,
`\r`, and `\t`; unknown escapes and raw control characters are errors. Raw Unicode
is supported; Unicode escape syntax is outside this version.

Values are signed 64-bit integers, booleans, or strings. Literal magnitudes range
from zero through `i64::MAX`; negative numbers use unary minus. Consequently the
direct spelling `-9223372036854775808` fails literal parsing; the minimum value
can be computed as `-9223372036854775807 - 1`. Arithmetic is integer-only
and checked, including unary negation. Division truncates toward zero; remainder
has the dividend's sign. Zero divisors and overflow are evaluation errors.
Equality requires matching operand types; ordering requires integers; boolean
operators require booleans. An `if` condition must be boolean, including in an
unselected enclosing branch. Both conditional branches must have the same type.
There is no truthiness, implicit coercion, string concatenation, or floating point.
Interpolation renders integers in decimal, booleans as `true`/`false`, and strings
as their contents.

`if` evaluates only its chosen branch. `&&` and `||` short-circuit. Other operands
evaluate left to right. Both branches are parsed and type-checked even when a
condition is constant; runtime errors in an unselected branch are not evaluated.

## Dice and text

`roll("1d4")` accepts only `[N]dS`: ASCII decimal count (default 1), lowercase
`d`, and ASCII decimal sides, with no internal whitespace. Count must be 1–1,000
and sides 1–1,000,000. Parse/check that form before delegating to the pinned
diceman dependency using the current roller RNG. Require a numeric `i64` outcome;
an unexpected nonnumeric outcome is an evaluation error. Validation never samples.
Dynamic dice strings, arithmetic inside the dice string, Fudge/digit dice,
explosions, keep/drop, and calls to other functions are rejected in `roll()`.
Existing table dice and ordinary `{dice}` retain their own syntax.
All dice literals are parsed and range-checked during preparation, even in
unselected branches: `if false then roll("0d6") else 1` fails validation;
`if false then 1 / 0 else 1` is well-typed and evaluates to 1.

This restriction keeps computed arithmetic in FateScroll's checked evaluator.
The pinned diceman evaluator uses unchecked arithmetic for its binary operators
(discovery E7); accepting its full grammar here would undermine that guarantee.
Write `roll("2d6") * 10`, not `roll("2d6x10")`.

Dice calls are allowed in binding expressions, including conditional branches.
They are prohibited anywhere in `{= ...}` expressions, including unselected
branches: authors give a random value a name before interpolating it.

`{= expression}` is a reserved, strict template marker. A scanner finds its
closing brace outside string literals; braces inside quoted expression strings
are ordinary characters. Missing closing braces, empty expressions, and invalid
expressions are errors. `{{=` emits the literal opening `{=` and prevents that
opening from being evaluated. This escape is recognized before `{=`. Ordinary
text has no other added escape rules.

Existing `{dice}` text remains available. Its current tolerant behavior is the
proposal for that syntax: failed or nonnumeric dice expressions stay literal.
This is a proposed format-preservation decision for Jerry to approve before
implementation, not authorization to build migration or compatibility layers.
Previously literal `{= ...}` and `{{=` acquire meaning; document this reserved
syntax change explicitly. Do not advertise universal byte-for-byte compatibility.

Reserved `{{=` and `{=` openings take precedence throughout source text, even
inside an ordinary-looking brace candidate. Partition source at those openings,
consume strict segments with the quote-aware scanner, and use the existing
nonoverlapping ordinary-dice matcher within the remaining spans. For example,
`{note {= value}}` with value 2 renders `{note 2}`, while `{{1d6}}` remains literal
under the ordinary matcher's current consumption rule. `{{= value}` renders
`{= value}` literally. An escaped opening's emitted text is never matched again.

The scanner emits literal, ordinary-dice, and expression segments in source order.
Use the existing dice interpolation behavior for ordinary text segments. Render
each source segment once. Never rescan generated output: a binding containing
the string `"{1d6}"` emits those characters without rolling another die.

## Evaluation lifecycle

1. Select a table entry using the existing lookup and reroll rules.
2. Parse and type-check that entry's bindings and template without using the RNG.
3. Set `value` to the selected lookup. Evaluate bindings once in list order,
   including unused bindings; store their values. Lazy branches may skip dice.
4. Render text once, left to right. `text: null`/absent stays absent; bindings
   still evaluate if declared. An empty string remains an empty string in core.
5. Roll chains in their existing order using the same RNG, with fresh environments.

Validation checks every result entry, not only one sampled outcome. No validation
or parsing step rolls dice. Rendering errors abort the roll before any child is
rolled; no partial `RollResult` is returned. Consumed RNG state is not rolled back.
The result tree retains `table_name`, `roll`, `text`, and `children`; evaluated
bindings are not added to the public JSON format.

## Implementation boundaries

- `models.rs`: `ResultBinding { name: String, value: String }`; add
  `ResultEntry.bindings: Vec<ResultBinding>` serialized as `let`, with an empty
  default and omitted when empty. Retain `text: Option<String>` and chain shape.
- `expression.rs`: private syntax tree, expression parser, type checker, and
  evaluator. No registry access, I/O, clocks, globals, or JS execution.
- `result_text.rs`: compile bindings and template segments into a temporary
  `PreparedResult`, check names/order/types, then render with lookup and RNG.
  One entry-level boundary used by validator and roller. No registry cache or
  serialized AST in this version; compiling twice is acceptable at this scale.
- `error.rs`: contextual validation and rolling errors with table identifier,
  result index (displayed one-based), binding index/name or text location, and
  expression byte offset/reason. Byte offsets refer to decoded expression strings,
  not YAML file columns. Do not claim source-line precision not retained by serde.
- `validator.rs` and `roller.rs`: call the shared preparation/rendering boundary.
  Preserve public roll entry points and normal/quiet/JSON rendering contracts.
- `display.rs`: `show` prints declared bindings in order and source template,
  without evaluation. Expressions become part of the table explanation.
- WASM and Table Forge: Rust remains the only evaluator. Carry source bindings
  through parsed JSON, draft state, editing, autofill, YAML, and zip.

Use a small recursive-descent parser with explicit depth checks. No additional
runtime dependency is proposed. Expression source is limited to 4,096 UTF-8 bytes,
syntax-tree depth to 64, and bindings per result to 128. Apply limits before
recursive traversal, including long left-associated operator trees. For entries
using bindings or strict markers, source and rendered text are each limited to
65,536 UTF-8 bytes; enforce output size during append, before allocating beyond
the budget. These bound this feature's parser/evaluator, not diceman's underlying
dice workload. Do not change existing dice resource policy as part of this issue.
The depth limit applies independently to syntactic nesting and AST node depth,
with the root at depth 1. Group parentheses, unary operators, and conditional
nesting increase syntactic depth; precedence-function calls alone do not. Check
before descending past the limit even when parentheses will be omitted from the
AST. Check constructed node depth before attaching it to a parent or traversing it.

## Table Forge authoring

Each result card gains an optional **Values** section with ordered **Name** and
**Expression** rows, add/remove controls, and move-up/move-down controls. The
existing text field accepts `{= ...}` and offers one short example. Reordering
can invalidate dependencies; the normal validation pane reports that error.

Invalid expression strings remain editable and round-trip through import/export;
structurally malformed binding YAML fails parsing. Avoid frontend evaluation or
duplicated syntax validation. Import assigns fresh editor IDs to binding rows;
autofill preserves values with the rest of each retained result. No result
duplication control is introduced. Edits clear stale
roll previews through the existing store/engine flow.

Raise the persisted draft version from 1 to 2. For version 1, validate the saved
document using the existing shape checks, copy it, and add `bindings: []` to each
result while preserving all other data and editor IDs. Validate nested table and
result container shapes before traversing; malformed version-1 data follows the
existing discard path without throwing. Unknown versions retain the existing
discard policy. Version-2 drafts preserve binding names, sources, order, and IDs.
This narrowly scoped migration is explicitly approved by Jerry; no general
migration framework or support for other versions is included.

## Scope limits and completion gate

No conditional chains, computed table IDs, structured output records, user
functions, loops, mutation, external inputs, cross-table variables, or persistent
state. Conditions choose values and wording within a selected result. The broader
ideas in kmgh remain possible follow-up work, not hidden requirements here.

Completion requires core unit tests, real CLI integration tests, WASM bridge
tests, and Table Forge import/edit/export/roll tests. Cover the acceptance example,
deterministic RNG ordering and reuse, lazy evaluation, lexical scope, parser and
arithmetic errors, limits, ordinary dice preservation, and browser round-tripping.
Run format/lint checks and the final branch review before integration.

## Evidence

Code discovery is recorded in `.scratchpad/20260904-expression-discovery-kmgh.md`.
Key baseline sources: `fatescroll-core/src/models.rs:49`,
`fatescroll-core/src/roller.rs:237`, `fatescroll-core/src/roller.rs:312`,
`webui/src/import/mapDrafts.ts:36`, and `docs/authoring-guide.md` (Dice Interpolation).
This document specifies proposed behavior; none of its feature examples has been
implemented or claimed to run on the baseline.
