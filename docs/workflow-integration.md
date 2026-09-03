# Workflow Integration

FateScroll integrates best when it's **invited at setup time and automatic at use time**. You configure it once — an alias, a template, a Makefile target — and from then on it shows up without you having to think about it.

The test for whether an integration is a nudge or an imposition: can you ignore the random output and proceed without extra steps? If yes, it's a nudge. If no, it's an imposition. The patterns below all pass that test.

---

## Shell Alias

The simplest integration. Add this to your `.bashrc` or `.zshrc`:

```sh
alias review='fatescroll roll --collection ~/tables/review review.prompts.code-review'
```

Running `review` before opening a diff gives you a random lens. You can take it or leave it — nothing downstream depends on it.

---

## Git Alias

A `git review` alias that rolls a lens and then opens the diff, combined into one command:

```sh
git config --global alias.review '!fatescroll roll --collection ~/tables/review review.prompts.code-review && git diff'
```

After setup, `git review` becomes a natural part of the review flow. The random output appears at the top of your terminal session, and then the diff follows immediately. You read the lens, or you scroll past it — either way, the diff is right there.

For a PR review variant that opens the diff against main:

```sh
git config --global alias.pr-review '!fatescroll roll --collection ~/tables/review review.prompts.code-review && git diff main...'
```

---

## PR Template Placeholder

The lowest-friction team integration. Add a section to `.github/PULL_REQUEST_TEMPLATE.md` that the author fills in before submitting:

```markdown
## Review Focus

<!-- Run: fatescroll roll --collection ~/tables/review review.prompts.code-review -->
<!-- Paste the result here, or delete this section if you prefer not to. -->
```

When an author runs the command and pastes the output, reviewers get a suggested angle without being required to use it. The "or delete this section" clause keeps it genuinely optional — no one is blocked by a skipped roll.

Sample output in a filled-in PR description:

```
## Review Focus

Code Review Prompt
  Angle (rolled 2): Naming and conceptual clarity
  Blind Spot (rolled 4): What invariant does this rely on that isn't documented?
```

---

## Makefile / justfile Target

For multi-step workflows where you want the lens before running linters or opening the diff:

**Makefile:**
```makefile
review:
	fatescroll roll --collection ~/tables/review review.prompts.code-review
	git diff
	make lint
```

**justfile:**
```just
review:
    fatescroll roll --collection ~/tables/review review.prompts.code-review
    git diff
    just lint
```

Running `make review` (or `just review`) rolls the lens, shows the diff, and runs the linter in sequence. The lens output is the first thing you see; everything else follows.

---

## What Not to Do: CI Auto-Comment

A CI job that automatically posts a review lens as a PR comment might seem like a convenient automation, but it fails the "invited" test. The reviewer didn't ask for it, they can't skip it without it cluttering the thread, and it shifts the pattern from opt-in to opt-out. Team members who find it unhelpful will start ignoring the comment entirely, which removes its value for everyone.

If you want reviewers to use random lenses, put the integration in their local setup — an alias or a git config — and let the PR template be a lightweight prompt for authors, not an automated imposition on reviewers.

---

## Scripting: Coming Features

FateScroll outputs human-readable text by default. Two flags improve scripting integration:

- `--json` — emit structured output for piping into other tools
- `--quiet` — suppress table names and metadata, output only the result text of every roll node, one per line (`--quiet` conflicts with `--json`)

With these, integrations like populating a variable in a shell script or feeding output to a formatter are straightforward.

---

## Systematic Coverage with deckbox

FateScroll rolls with replacement — every roll is independent, so you might see the same lens twice in a row and go weeks without seeing another. That's fine for casual use, but if you want to ensure every perspective gets covered before repeating (draw-without-replacement, like shuffling a deck), FateScroll's companion tool handles that case.

[deckbox](https://github.com/snits/deckbox) maintains stateful draws from a pool so that the same result doesn't appear twice until the pool is exhausted. Use FateScroll when independence is fine; use deckbox when systematic coverage matters.
