# Start a fresh Table Forge collection

## Problem

Table Forge persists the working document in browser `localStorage`. That
keeps drafts safe across reloads, but it also means the last document is
automatically restored on the next session. The UI currently has no way to
discard that restored document without importing another collection.

## Decision

Add a `Start new collection` command to the existing `Open collection` menu.
The command is an explicit discard boundary:

1. Ask for confirmation because the current editor contents are unexported
   work.
2. Reset the store to `initialState()`.
3. Remove the persisted Table Forge document from `localStorage`.

After confirmation, the editor shows the empty state and the next browser
startup begins with a blank collection instead of restoring the discarded one.

## Boundaries

- Imported collections continue to replace the current document as they do
  today.
- The command does not change the persisted schema or rehydration validation.
- The command is available from the existing open menu so users do not need a
  second navigation surface to leave a loaded collection.
- Confirmation is always shown for this destructive command, including when
  the current editor is already empty; this keeps the action's meaning
  predictable and avoids adding draft-detection rules to the UI.

## Verification

- Store behavior proves the document resets and the persisted key is removed.
- Header behavior proves the command is visible in the open menu, confirms,
  and clears the current document.
- The existing web UI test suite and build remain green.
