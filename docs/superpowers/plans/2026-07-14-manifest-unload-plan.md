# Start a fresh Table Forge collection

## Task 1: Add the store reset action

1. Add a public `startNewCollection()` action to `ForgeState`.
2. Reset document fields and selection to `initialState()`.
3. Clear the Zustand persistence key through the store's persistence API.
4. Add a store test that seeds a document, invokes the action, and verifies
   initial state plus absent persisted storage.

## Task 2: Expose the reset action in the open menu

1. Add a `Start new collection` button to the existing open menu.
2. Confirm before calling `startNewCollection()`.
3. Add a HeaderBar integration test proving the menu action resets the store
   and does not leave the document in storage.

## Task 3: Verify and land

1. Run focused web UI tests, then the full web UI test suite and build.
2. Review the implementation and commit the branch with sign-off.
3. Run roborev for the commit and address relevant findings.
4. Rebase the worktree branch onto local `main`, merge it back with `--no-ff`,
   and close kata issue `bg8c` with the merge commit and verification evidence.
