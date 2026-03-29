# Use Cases: FateScroll Beyond RPGs

FateScroll is a structured randomness engine for creative and evaluative work. The examples below show how its features — compound tables, chaining, and dice interpolation — serve workflows that have nothing to do with dungeons or dragons.

Each example is a complete, copy-pasteable collection. Save the YAML files in a directory alongside a `manifest.yaml`, then run `fatescroll validate` to confirm the structure before rolling.

---

## 1. Code Review Prompts

**Scenario:** A compound table generates a two-axis review prompt: one axis picks the *Angle* (what you're examining), the other picks the *Blind Spot* (what you're probably missing). Rolling both at once forces combinations you wouldn't choose deliberately.

`manifest.yaml`:
```yaml
name: Code Review Prompts
version: "1.0"
namespace: review
directories:
  - path: prompts
    namespace: review.prompts
```

`prompts/code-review.yaml` (compound table):
```yaml
name: Code Review Prompt
type: compound
tags:
  - code-review
  - engineering
tables:
  - angle
  - blind-spot
```

`prompts/angle.yaml`:
```yaml
name: Angle
type: simple
tags:
  - code-review
roll: 1d6
results:
  - min: 1
    max: 1
    text: Error handling and failure paths
  - min: 2
    max: 2
    text: Naming and conceptual clarity
  - min: 3
    max: 3
    text: State mutation and side effects
  - min: 4
    max: 4
    text: Testability and seams for mocking
  - min: 5
    max: 5
    text: Performance under realistic load
  - min: 6
    max: 6
    text: Security assumptions and trust boundaries
```

`prompts/blind-spot.yaml`:
```yaml
name: Blind Spot
type: simple
tags:
  - code-review
roll: 1d6
results:
  - min: 1
    max: 1
    text: "What would you change if you had to debug this at 3am?"
  - min: 2
    max: 2
    text: What does this code assume about its callers that isn't enforced?
  - min: 3
    max: 3
    text: Where does this break if the happy path takes twice as long?
  - min: 4
    max: 4
    text: What invariant does this rely on that isn't documented?
  - min: 5
    max: 5
    text: Which of these decisions will feel arbitrary to the next person here?
  - min: 6
    max: 6
    text: What would make this obviously wrong at a glance six months from now?
```

**Sample output:**
```
Code Review Prompt
  Angle (rolled 3): State mutation and side effects
  Blind Spot (rolled 1): What would you change if you had to debug this at 3am?
```

---

## 2. Retrospective Prompt Generator

**Scenario:** A compound table for team retrospectives combines a *Theme* (what to reflect on) with a *Framing* (how to approach the reflection). The combination surfaces angles a team wouldn't naturally land on, especially after several retros with the same format.

`manifest.yaml`:
```yaml
name: Retrospective Prompts
version: "1.0"
namespace: retro
directories:
  - path: prompts
    namespace: retro.prompts
```

`prompts/retro-prompt.yaml` (compound table):
```yaml
name: Retrospective Prompt
type: compound
tags:
  - retrospective
  - team
tables:
  - theme
  - framing
```

`prompts/theme.yaml`:
```yaml
name: Theme
type: simple
tags:
  - retrospective
roll: 1d6
results:
  - min: 1
    max: 1
    text: Coordination and handoffs between people
  - min: 2
    max: 2
    text: Decisions we made under uncertainty
  - min: 3
    max: 3
    text: Work that took longer than expected
  - min: 4
    max: 4
    text: Something we avoided talking about
  - min: 5
    max: 5
    text: A tool or process that helped or hurt
  - min: 6
    max: 6
    text: How we handled an interruption or surprise
```

`prompts/framing.yaml`:
```yaml
name: Framing
type: simple
tags:
  - retrospective
roll: 1d4
results:
  - min: 1
    max: 1
    text: "What did we do well here, and what made it possible?"
  - min: 2
    max: 2
    text: "If we could replay this, what's the one thing we'd change?"
  - min: 3
    max: 3
    text: What would an outside observer have noticed that we didn't?
  - min: 4
    max: 4
    text: What system or habit would make this better by default next time?
```

**Sample output:**
```
Retrospective Prompt
  Theme (rolled 4): Something we avoided talking about
  Framing (rolled 2): If we could replay this, what's the one thing we'd change?
```

---

## 3. Design Evaluation Lenses

**Scenario:** Each lens in Jesse Schell's *The Art of Game Design* comes with a set of probing questions. This table picks a lens and immediately chains to a follow-up question, so each roll hands you both the frame and the first thing to ask inside it.

`manifest.yaml`:
```yaml
name: Design Lenses
version: "1.0"
namespace: design
directories:
  - path: lenses
    namespace: design.lenses
```

`lenses/lens.yaml`:
```yaml
name: Design Lens
type: simple
tags:
  - design
  - evaluation
roll: 1d6
results:
  - min: 1
    max: 1
    text: "Lens of Essential Experience"
    chain:
      - lens-question
  - min: 2
    max: 2
    text: "Lens of Surprise"
    chain:
      - lens-question
  - min: 3
    max: 3
    text: "Lens of Fun"
    chain:
      - lens-question
  - min: 4
    max: 4
    text: "Lens of Curiosity"
    chain:
      - lens-question
  - min: 5
    max: 5
    text: "Lens of Endogenous Value"
    chain:
      - lens-question
  - min: 6
    max: 6
    text: "Lens of Problem Solving"
    chain:
      - lens-question
```

`lenses/lens-question.yaml`:
```yaml
name: Lens Question
type: simple
tags:
  - design
  - evaluation
roll: 1d6
results:
  - min: 1
    max: 1
    text: What is essential about this experience, and what can be removed without losing it?
  - min: 2
    max: 2
    text: Does this feel different each time, or does it reveal a pattern too quickly?
  - min: 3
    max: 3
    text: When someone smiles or laughs here, what caused it?
  - min: 4
    max: 4
    text: What question does this raise in the user's mind, and are we answering it?
  - min: 5
    max: 5
    text: Does the user feel ownership over this, and does that feeling match the mechanics?
  - min: 6
    max: 6
    text: What problem is the user solving, and is that problem interesting on its own terms?
```

**Sample output:**
```
Design Lens (rolled 2): Lens of Surprise
  Lens Question (rolled 5): Does the user feel ownership over this, and does that feeling match the mechanics?
```

---

## 4. Oblique Strategies

**Scenario:** Brian Eno and Peter Schmidt's *Oblique Strategies* are terse creative provocations designed to unstick a creative process. This table gives you one at random when you're blocked, overworked, or too close to the thing you're making.

`manifest.yaml`:
```yaml
name: Oblique Strategies
version: "1.0"
namespace: oblique
directories:
  - path: strategies
    namespace: oblique.strategies
```

`strategies/strategy.yaml`:
```yaml
name: Oblique Strategy
type: simple
tags:
  - creative
  - oblique-strategies
roll: 1d8
results:
  - min: 1
    max: 1
    text: Use an old idea
  - min: 2
    max: 2
    text: State the problem in words as clearly as possible
  - min: 3
    max: 3
    text: Only one element of each kind
  - min: 4
    max: 4
    text: What would your closest friend do?
  - min: 5
    max: 5
    text: "Do nothing for as long as possible"
  - min: 6
    max: 6
    text: Remove ambiguities and convert to specifics
  - min: 7
    max: 7
    text: Work at a different speed
  - min: 8
    max: 8
    text: Emphasize the flaws
```

**Sample output:**
```
Oblique Strategy (rolled 7): Work at a different speed
```

---

## 5. Journaling Prompts

**Scenario:** A compound table for solo reflective practice combines a *Domain* (the area of life to examine) with a *Prompt Style* (the mode of inquiry). The cross-product of six domains and four styles gives twenty-four combinations — enough variety to sustain a daily practice without feeling repetitive.

`manifest.yaml`:
```yaml
name: Journaling Prompts
version: "1.0"
namespace: journal
directories:
  - path: prompts
    namespace: journal.prompts
```

`prompts/journal-prompt.yaml` (compound table):
```yaml
name: Journal Prompt
type: compound
tags:
  - journaling
  - reflection
tables:
  - domain
  - prompt-style
```

`prompts/domain.yaml`:
```yaml
name: Domain
type: simple
tags:
  - journaling
roll: 1d6
results:
  - min: 1
    max: 1
    text: Work and creative output
  - min: 2
    max: 2
    text: Relationships and what I owe people
  - min: 3
    max: 3
    text: Energy, attention, and what drains them
  - min: 4
    max: 4
    text: A belief I haven't examined lately
  - min: 5
    max: 5
    text: Something I'm avoiding
  - min: 6
    max: 6
    text: Progress toward something I said matters to me
```

`prompts/prompt-style.yaml`:
```yaml
name: Prompt Style
type: simple
tags:
  - journaling
roll: 1d4
results:
  - min: 1
    max: 1
    text: "Write for {2d4} minutes without stopping or editing"
  - min: 2
    max: 2
    text: List three things that are true, and one you're not sure about
  - min: 3
    max: 3
    text: Describe this from the perspective of someone who knows you well
  - min: 4
    max: 4
    text: What would change if you took this more seriously for one week?
```

**Sample output:**
```
Journal Prompt
  Domain (rolled 5): Something I'm avoiding
  Prompt Style (rolled 1): Write for 6 minutes without stopping or editing
```

---

## Gallery

FateScroll's table format adapts to any domain that benefits from structured, repeatable randomness: teaching exercises where a random combination of constraints forces students to solve problems they'd never set for themselves; photography prompts that cross-combine subject, lighting condition, and compositional rule; music practice sessions that draw from a pool of scales, tempos, and technique focuses to break up rote repetition; design evaluation workflows where a lens and a follow-up question arrive together, reducing the gap between noticing and asking.
