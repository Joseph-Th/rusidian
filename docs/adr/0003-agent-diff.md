# ADR 0003 — Agent action diff representation

Status: accepted · Date: 2026-06-25

## Context

Agent actions must record what changed so users can inspect and roll back modifications
(AGENTS.md "Versioning and Audit Rules"). Each `AgentAction` needs a `diff` field that
captures the before/after state. The question is how to represent it.

Two approaches:
1. **Full snapshots**: store complete serialized state before and after.
2. **Structured patches**: store minimal JSON patch or similar transformation.

## Decision

Use **full snapshots** as the initial representation.

Store `diff` as a JSON object with `before` and `after` keys, each holding the complete
object state serialized as JSON. Example:

```json
{
  "before": { /* complete Block before edit */ },
  "after": { /* complete Block after edit */ }
}
```

## Rationale

1. **Simplicity**: snapshots require no patch format understanding, no transformation
   logic, no risk of patch failures during rollback.
2. **Clarity**: anyone inspecting the audit log sees exactly what the object looked like
   at each step.
3. **Correctness**: full state makes rollback trivial — just deserialize `before` and
   write it back.
4. **Debuggability**: diffs are human-readable JSON; no need to understand patch semantics.
5. **Local-first**: this is a personal knowledge workbench with local storage. The size
   overhead of full snapshots is acceptable.

## Consequences

- `AgentAction` diffs are larger than they could be with patches (trade-off accepted).
- Rollback is simple: deserialize `diff.before` and persist it.
- No need to implement patch application logic in D2; can be added later if needed.
- The audit trail is fully human-inspectable.

## Alternatives considered

- **JSON Patch (RFC 6902)**: industry standard, compact, widely understood. Rejected
  because patch application is complex and error-prone; a bug in rollback could lose
  data. Not worth the size savings for local-first storage.
- **Unidirectional before-only**: rollback would need to compute the "undo". Rejected
  because it's less clear and more error-prone than storing both states.
- **Event sourcing / CRDT**: too much complexity for the current scope. Revisit if
  collaboration / sync is added.
