# Cross-Namespace Chain Reference Resolution Design

## Overview

Add global bare id fallback to `Registry::resolve` so chain references work across namespace boundaries without requiring fully qualified ids.

Covers bead fatescroll-s33.

## Problem

Current resolution order:
1. Relative: `current_namespace.reference` (same namespace)
2. Absolute: `reference` as FQID

This means a table in `campaign1.npc.quick-npc` can reference `npc-occupation` (same namespace) but cannot use the bare id to reach `campaign1.encounters.wolf-count` — it would need the full FQID. When tables are shared across collections with different namespace layouts, bare references break because the FQID changes per collection.

## Design

Add a third resolution step: **global bare id fallback**.

**Resolution order:**
1. Relative: `current_namespace.reference`
2. Absolute: `reference` as FQID
3. Global: search all tables where FQID ends with `.reference` — return if exactly one match

If the global step finds multiple matches, the reference is ambiguous and returns None (same as not found). The user gets the existing "unresolved reference" error and can disambiguate with a FQID.

## Implementation

Single change to `Registry::resolve` in `src/registry.rs`. Linear scan of `self.tables` — fine at fatescroll's scale.

## Scope

- Global bare id fallback only
- No ambiguity-specific error messages (YAGNI — add if users hit it)
- No changes to resolve() return type (stays `Option`)
- No changes to callers (validator, roller)
