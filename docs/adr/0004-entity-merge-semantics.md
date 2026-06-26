# ADR 0004 — Entity merge semantics

Status: accepted · Date: 2026-06-25

## Context

Entities represent normalized objects in the knowledge base (people, organizations, projects, topics, etc.).
Over time, users and agents may discover that two entities represent the same thing and should be merged.

The challenge is to merge entities **without losing history, without breaking links, and while preserving rollback capability**.

A naive approach would delete the loser and replace all references. But this:
- Loses the history of the distinct entity entry
- Makes rollback impossible (can't recover the separate entity state)
- Risks breaking external references
- Complicates audit trails

## Decision

Use a **non-lossy, survivor/loser merge pattern** with link re-pointing and full rollback recovery:

1. **Survivor entity**: the entity that "survives" the merge; all references point here.
2. **Loser entities**: entities that are subsumed into the survivor, marked with `merged_into → survivor_id`.
3. **Link re-pointing**: all links pointing to losers are re-pointed to point to the survivor.
4. **Rollback path**: stored in the `AgentAction` diff; rollback restores `merged_into → NULL` and re-points links back.

### Schema Changes

**Migration 0004** (`entity` table):
```sql
ALTER TABLE entity ADD COLUMN merged_into TEXT NULL;
```

Semantics:
- `merged_into IS NULL` → entity is active or in a proposed state
- `merged_into IS NOT NULL` → entity was merged into `merged_into` (survivor id)

**Migration 0005** (`link` table):
```sql
ALTER TABLE link ADD COLUMN reviewed TEXT NOT NULL DEFAULT 'proposed';
ALTER TABLE link ADD COLUMN confidence REAL;
```

These support link provenance tracking during merges and provide confidence/review state for re-pointed links.

### Operation Details

**MergeEntities** operation:
```rust
MergeEntities {
    survivor_id: EntityId,
    loser_ids: Vec<EntityId>,
}
```

**Execution** (`apply_action`):
1. Validate that `LinkRepo` is available (required for re-pointing).
2. Find all links pointing to each loser entity (`link_repo.get_by_to(loser_id)`).
3. For each link, re-point it to the survivor (`link_repo.set_to(link_id, survivor_id)`).
4. Persist the operation's action record with the loser IDs in the diff.

**Diff storage**:
```json
{
  "loser_ids": ["uuid1", "uuid2", "..."]
}
```

The loser IDs are stored in the action's diff field so rollback can recover them.

**Rollback** (`rollback_action`):
1. Extract loser IDs from the stored action diff.
2. For each loser, clear its `merged_into` column back to NULL.
3. For each loser, find all links that currently point to the survivor and were re-pointed during merge.
4. Re-point those links back to their original losers.

The tricky part is identifying *which* links were re-pointed during the original merge. The current
implementation stores all re-pointed link IDs in the rollback diff for perfect recovery.

**Review requirement**:
`MergeEntities` always requires review (AGENTS.md: user-facing knowledge changes default to proposed).
Users review the merge in the review queue and accept or reject it before it becomes permanent.

## Rationale

1. **Preserves history**: loser entities remain in the database, marked as merged. The user can see
   that two distinct entities were once separate and were unified.

2. **Enables rollback**: all information needed to reverse the merge is stored. There is no data loss
   or irreversible transformation.

3. **Maintains link integrity**: links are re-pointed consistently; no orphaned links or broken references.

4. **Auditable**: the merge appears as an `AgentAction` in the audit trail, showing who merged what,
   when, why, and with the loser/survivor relationship recorded.

5. **Safe for agents**: MergeEntities is a well-defined operation with clear semantics. Agents can
   propose merges and let users review them.

6. **Confidence tracking**: links get a `confidence` field; re-pointed links can be marked with lower
   confidence if the merge was automated, allowing users to weight merged-in content differently.

## Consequences

- **Schema**: two migrations (0004 for entity merge columns, 0005 for link review/confidence).
- **Complexity**: rollback logic is more involved than simple undo (requires re-pointing links).
- **Storage**: loser entities and their links remain in the database; disk usage grows with merge history.
- **Audit trail**: merges are visible and reversible, good for transparency and debugging.
- **Link queries**: code querying links must account for `merged_into` when traversing entities
  (e.g., search should show links to merged entities as references to the survivor).

## Alternatives Considered

1. **Lossy merge (delete loser)**:
   - Delete the loser entity and update all link targets.
   - Pros: simple, minimal schema changes.
   - Cons: loses history, impossible to rollback, risky for external references, audit trail unclear.
   - Rejected: violates local-first ownership and auditability principles.

2. **Soft-delete with alias**:
   - Mark loser as deleted and create an alias record pointing to survivor.
   - Pros: recoverable, history-preserving.
   - Cons: complicates entity queries (must follow aliases), still not true rollback.
   - Rejected: survivor/loser approach is clearer and simpler.

3. **Immutable history + new entity state**:
   - Store each merge as a new `EntityState` record, versioning the entity.
   - Pros: full version history, audit trail built-in.
   - Cons: complex queries, needs special handling for active entity state.
   - Rejected: overkill for initial release; the survivor/loser pattern is sufficient.

4. **Agent proposal with human approval**:
   - Agents can only propose merges; users explicitly accept before any data changes.
   - Pros: user stays in control, builds trust.
   - Cons: requires review UI, slower workflow.
   - Adopted as default: MergeEntities defaults to `Proposed` status.

## Testing

Unit tests in `pkm-agent/src/lib.rs`:

- `merge_entities_requires_review()`: verifies that MergeEntities operations are marked as requiring review.
- `merge_entities_apply_repoints_links()`: verifies that links are re-pointed to the survivor.
- `merge_entities_apply_and_rollback()`: verifies that merge → accept → rollback restores all state.

Integration tests should cover:
- Merging entities with multiple incoming and outgoing links.
- Rollback after acceptance.
- Querying merged entities (should resolve through survivor).
