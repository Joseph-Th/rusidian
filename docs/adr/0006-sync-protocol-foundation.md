# ADR 0006 — Sync Protocol Foundation

Status: proposed · Date: 2026-06-26

## Decision

Establish a **version-vector-based sync protocol** for detecting and resolving conflicts
when two copies of the same object diverge.

The sync eligibility model uses:
1. **Version numbers** (monotonically increasing per object)
2. **Updated timestamps** (wall-clock time of last change)
3. **Conflict detection** (concurrent edits with same parent version)
4. **Manual merge** (conflicts require user review before acceptance)

## Context

H2 added `version` and `updated_at` fields to all domain objects (Source, Note, Block, Entity, Link, View).
Now we need a protocol to determine:

1. When an object can be safely synced (no conflict)
2. When two versions conflict (concurrent edits)
3. How to merge conflicting versions (user-driven, not automatic)

This is necessary for:
- Multi-device sync (user edits on desktop and phone)
- Collaborative features (shared vaults)
- Audit trails (who changed what when)

## Proposed Design

### Sync Eligibility Rules

An object is **sync-eligible** if:
- It has valid `version` and `updated_at` metadata
- The version is >= 1
- Its version history is complete (no gaps in sequence)

An object **can be merged** if:
- Both versions have the same base (same version before edit)
- OR timestamps allow a clear causality (one is clearly earlier)
- OR user explicitly accepts a merge

An object **has a conflict** if:
- Two versions diverge from the same parent version
- Both have independent edits (timestamps don't establish clear causality)
- Example: version 2 created on device A at 10:00:00
            version 2 created on device B at 10:00:05
            (both claim to be the next version after 1)

### Conflict Detection Algorithm

```
detect_conflict(obj_a, obj_b):
  if obj_a.version != obj_b.version:
    return false  # Different versions, not a conflict yet

  if obj_a.version == 1:
    return true   # Both claim to be initial version: conflict

  # Check if both diverged from same parent
  if obj_a.updated_at < obj_b.updated_at:
    return false  # A was edited first, B came after: B won

  if obj_a.updated_at > obj_b.updated_at:
    return false  # B was edited first, A came after: A won

  # Same version, same timestamp: concurrent edit
  return true
```

### Merge Resolution

Conflicts are resolved via:
1. **Last-write-wins** (automatic): If timestamps differ, newer version takes precedence
2. **Manual review** (user-driven): If timestamps are identical or within milliseconds,
   the UI presents both versions for user merge

### Transport Contract (H3c)

Sync requests carry:
- object_id, object_type
- current_version, updated_at (local state)
- remote_version, remote_updated_at (expected state)
- proposed_content (new content if accepting local)

Sync responses indicate:
- accepted (merged)
- conflict (requires user review)
- stale_request (local version is old)

## Alternatives Considered

1. **CRDT-based (Operational Transform)**
   - Rejected: Complexity overkill for single-user (local-first) application.
   - Useful when automatic merge is desired; our model prefers reviewable changes.

2. **Three-way merge (git-style)**
   - Rejected: Requires storing common ancestor; adds schema complexity.
   - Could revisit when collaborative features are prioritized.

3. **Timestamp-only (no version numbers)**
   - Rejected: Timestamps can collide; version numbers provide clearer intent.

## Consequences

1. **Schema is stable** (version/updated_at already exist from H2).
2. **Sync eligibility checks are lightweight** (compare version + timestamp).
3. **Conflicts require user action** (prevents data loss).
4. **Audit trail is clear** (version history shows causality).
5. **Multi-device sync is now possible** (detect divergence, prompt for merge).
6. **Collaborative features must be added carefully** (no automatic merges).

## Migration Impact

None. Version fields are already persisted. Sync is an opt-in feature.

## Next Steps

1. **H3d**: Implement `SyncEligibility` trait and conflict detection.
2. **H3e** (future): Add transport layer (sync request/response types).
3. **H3f** (future): Add sync state machine (pending, synced, conflict).
