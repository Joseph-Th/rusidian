# AGENTS.md

## Project Identity

This project is a local-first, agent-native personal knowledge workbench.

It helps a user capture information, preserve source provenance, organize knowledge, retrieve context, and turn notes into clean, presentable views.

The product is built around one core loop:

1. Capture information.
2. Preserve the raw source.
3. Parse and structure it.
4. Propose summaries, entities, links, and metadata.
5. Let the user review and accept useful structure.
6. Promote accepted material into durable knowledge.
7. Retrieve knowledge with clear provenance.
8. Present information through clean views.
9. Let agents propose safe, auditable changes.
10. Keep everything exportable and recoverable.

Every feature should strengthen this loop.

## North Star

The system should feel like a calm, intelligent workspace for personal knowledge.

It should make it easy to answer:

* What have I collected?
* Where did it come from?
* What does it mean?
* What is connected?
* What needs review?
* What can be trusted?
* What changed?
* What should be surfaced now?
* How can this be presented clearly?

The product should help both humans and LLM agents work with knowledge without relying on fragile conventions, scattered files, or hidden prompt state.

## Core Product Principles

### 1. Local-first ownership

User data lives locally by default.

The system should work without a cloud account, remote database, or hosted service. Cloud sync, hosted inference, and collaboration may exist later, but the core application must remain useful offline.

User data must be durable, inspectable, exportable, and recoverable.

### 2. Structured knowledge as the foundation

The system stores knowledge as structured objects.

Markdown compatibility matters, but markdown is an export and editing format, not the entire product model.

The core model includes:

* sources
* notes
* blocks
* entities
* typed links
* metadata
* views
* agent actions
* review states
* provenance records

Agents and UI features should work with these objects directly.

### 3. Source provenance everywhere

Every derived piece of knowledge should be traceable back to where it came from.

The system must distinguish:

* raw source material
* user-authored notes
* extracted metadata
* generated summaries
* inferred links
* reviewed knowledge
* rejected suggestions

The user should always be able to inspect the source behind a claim, summary, link, or generated view.

### 4. Ingestion as a real pipeline

Capture is only the start.

The ingestion system should move information through explicit states:

* captured
* parsed
* cleaned
* indexed
* summarized
* classified
* linked
* awaiting_review
* promoted
* archived
* rejected
* failed

Raw source capture should be preserved. Cleanup, summaries, classifications, and links should be stored as derived artifacts.

The review queue is a first-class product surface.

### 5. Presentation-first views

The system should turn structured knowledge into clean, readable, useful views.

Important views include:

* reading queue
* review queue
* dossier
* project dashboard
* timeline
* source map
* person profile
* entity page
* decision log
* open questions
* action list
* briefing page

Views should be built from structured data, not hand-styled documents. The default presentation should be good enough that the user does not need to manually clean it up.

### 6. Agent-native operations

LLM agents are first-class users of the system.

Agents should interact with the knowledge base through typed operations, not by blindly rewriting files.

Good agent operations include:

* create_source
* parse_source
* create_note
* create_block
* update_block
* move_block
* create_entity
* merge_entities
* create_typed_link
* propose_typed_link
* attach_source_to_note
* propose_summary
* mark_reviewed
* create_view
* update_view
* search
* retrieve_context
* rollback_action

Every meaningful agent action should be inspectable, auditable, and reversible.

### 7. Reviewable automation

Automation should prepare useful structure. The user decides what becomes durable knowledge.

Agent-generated summaries, links, metadata, and entity merges should normally enter a proposed state before becoming accepted knowledge.

The system should make review fast, not optional.

### 8. Fast retrieval

The system should retrieve knowledge through multiple modes:

* exact text search
* fuzzy text search
* semantic search
* entity search
* source search
* date filtering
* type filtering
* review-state filtering
* project filtering
* typed link traversal
* citation-aware retrieval

Search results should clearly show whether content is raw, reviewed, generated, inferred, stale, or unreviewed.

### 9. Auditable change history

The system should keep a clear history of meaningful changes.

Agent actions and user actions should include enough information to understand:

* what changed
* who or what changed it
* when it changed
* why it changed
* what source or task caused the change
* how to roll it back

Reliability is part of the product.

### 10. Opinionated simplicity

The product should make strong design decisions.

Prefer coherent workflows over unlimited configuration. Prefer excellent built-in views over user-authored styling. Prefer typed objects over conventions. Prefer stable APIs over ad hoc integrations.

## Core Domain Model

### Source

A source is a captured piece of raw information.

Examples:

* web article
* PDF
* email
* screenshot
* audio transcript
* pasted text
* book excerpt
* imported markdown file
* meeting note
* document attachment

A source should contain:

* stable ID
* source type
* original content or file reference
* title
* author or origin when known
* captured timestamp
* source URL or locator when available
* extraction status
* review status
* derived artifacts
* linked notes, entities, and views

Raw source content should remain available even after summaries or notes are created from it.

### Note

A note is a durable knowledge object.

A note may contain blocks, metadata, typed links, source references, user-authored text, and generated sections. Notes should be easy to edit as markdown-compatible documents while still being represented internally as structured records.

A note should contain:

* stable ID
* title
* blocks
* metadata
* linked sources
* linked entities
* outgoing typed links
* backlinks
* creation timestamp
* modification timestamp
* author or actor history
* review state when relevant

### Block

A block is a stable, addressable unit inside a note or view.

Blocks allow safe partial edits. Agents should prefer block-level changes over full-note rewrites.

A block should contain:

* stable ID
* parent object
* block type
* content
* order
* source references when applicable
* author or actor metadata
* timestamps

Useful block types include:

* paragraph
* heading
* quote
* claim
* summary
* task
* list
* table
* code
* embed
* source excerpt
* generated section

### Entity

An entity is a normalized object the system can recognize, link, and retrieve.

Examples:

* person
* organization
* project
* topic
* location
* event
* book
* paper
* product
* claim
* decision

An entity should contain:

* stable ID
* type
* canonical name
* aliases
* description
* linked sources
* linked notes
* relationships
* confidence or review state when inferred

Entity merge operations must be reviewable and reversible.

### Typed Link

A typed link is a meaningful relationship between objects.

Use typed links to make knowledge traversable and agent-readable.

Initial link types:

* related_to
* cites
* supports
* contradicts
* summarizes
* derived_from
* mentions
* part_of
* depends_on
* decided_in
* assigned_to
* follows_up
* answers
* raises_question
* updates
* duplicates

A typed link should contain:

* stable ID
* source object
* target object
* link type
* confidence
* review state
* created by
* created timestamp
* supporting source when applicable

### View

A view is a presentation-first rendering of structured knowledge.

Views should help the user understand and use their information without manually arranging everything.

A view should contain:

* stable ID
* view type
* query or source objects
* layout model
* rendered sections
* filters
* sort order
* source references
* generated timestamp when applicable
* review state when generated

Important view types:

* reading_queue
* review_queue
* dossier
* project_dashboard
* timeline
* source_map
* entity_page
* person_profile
* decision_log
* open_questions
* action_list
* briefing_page

### Agent Action

An agent action is a proposed or applied operation by an LLM agent.

An agent action should contain:

* stable ID
* actor
* operation type
* target object
* input context
* before state or diff
* after state or proposed patch
* timestamp
* rationale
* status
* rollback reference
* related user request

Statuses:

* proposed
* accepted
* rejected
* applied
* reverted
* failed

Agent actions are product objects, not log noise. They are how the system remains trustworthy while using automation.

## Product Architecture

The architecture should separate durable knowledge, derived indexes, presentation views, and agent operations.

### Storage Layer

Responsible for:

* durable local objects
* schema migrations
* object IDs
* relationships
* version history
* raw source preservation
* exportable data

Expected direction:

* embedded local database
* explicit schema
* migrations
* structured JSON export
* markdown-compatible export

### Content Layer

Responsible for:

* note model
* block model
* markdown import/export
* source extraction
* source cleanup
* metadata extraction
* entity extraction
* summary generation
* link proposal

### Retrieval Layer

Responsible for:

* full-text indexing
* semantic indexing when available
* metadata filtering
* entity retrieval
* typed link traversal
* source-cited retrieval
* staleness and review-state awareness

### Agent Layer

Responsible for:

* typed operations
* operation validation
* change proposals
* diffs
* approvals
* rollback
* tool API
* eventual MCP-compatible interface

### Presentation Layer

Responsible for:

* note editor
* source viewer
* review queue
* search interface
* entity pages
* project dashboards
* timelines
* dossiers
* briefing pages
* agent change review

The presentation layer should consume structured objects and view models. It should not become the hidden source of truth.

## Expected Technical Direction

Until there is an explicit architecture decision, assume:

* backend language: Rust
* application model: local-first desktop app
* UI shell: Tauri or equivalent Rust-backed desktop shell
* storage: embedded local database
* indexing: local full-text index
* semantic search: optional local vector index after base retrieval is stable
* file watching: local filesystem watcher where useful
* agent API: local typed operation API
* export: markdown plus structured JSON
* import: markdown folders first, richer importers later

Remote services should be adapters, not foundations.

## Development Phases

### Phase 1: Foundation

Build the durable local application core.

Priorities:

* project structure
* domain types
* local database
* migrations
* note model
* block model
* markdown-compatible import/export
* basic editor
* basic search
* stable IDs
* versioning foundation

Success condition:

A user can create, edit, search, import, export, and recover basic notes locally.

### Phase 2: Knowledge Model

Build the structured knowledge layer.

Priorities:

* sources
* attachments
* source metadata
* entities
* typed links
* review states
* provenance records
* note-source relationships
* entity-note relationships

Success condition:

The system can distinguish raw source material, user notes, generated summaries, entities, links, and reviewed knowledge.

### Phase 3: Agent Safety Layer

Build the agent operation system.

Priorities:

* typed operations
* operation validation
* agent action log
* diff generation
* proposal status
* approval workflow
* rollback
* context retrieval API

Success condition:

An agent can propose useful changes without directly mutating arbitrary user content, and the user can inspect and reverse those changes.

### Phase 4: Ingestion Pipeline

Build capture and review.

Priorities:

* manual capture
* pasted text import
* markdown import
* web clip import
* PDF import
* source parsing
* cleanup
* summary proposal
* entity proposal
* link proposal
* review queue
* promotion flow

Success condition:

A user can capture a source, review system-generated structure, and promote selected material into durable knowledge.

### Phase 5: Presentation Views

Build the information-first interface.

Priorities:

* reading queue
* review queue
* dossier view
* timeline view
* project dashboard
* source map
* entity page
* person profile
* decision log
* briefing page

Success condition:

The user can turn structured knowledge into clean, useful views without manual formatting.

### Phase 6: Advanced Retrieval

Build deeper search and context assembly.

Priorities:

* hybrid search
* semantic search
* entity-aware retrieval
* typed relationship traversal
* source-cited answers
* stale-content detection
* contradiction surfacing
* unresolved question surfacing

Success condition:

The system retrieves relevant, provenance-aware context for both the user and agents.

### Phase 7: Expansion

Add broader product surfaces after the core loop works.

Possible additions:

* sync
* mobile capture
* full mobile editor
* public publishing
* collaboration
* visual workspace
* plugin API

Expansion features should preserve the core data model and agent safety layer.

## Agent Workflow

Every coding agent must follow this workflow.

### 1. Orient

Before changing code:

* Read this file.
* Identify the requested task.
* Identify which product phase it belongs to.
* Identify which core objects are affected.
* Inspect relevant existing files.
* Check for existing patterns before creating new ones.

### 2. Plan

Before editing:

* State the concrete change.
* Identify the smallest coherent implementation.
* Identify likely affected files.
* Identify data model or migration impact.
* Identify test impact.

### 3. Implement

While editing:

* Prefer small changes.
* Preserve existing architecture boundaries.
* Keep IO at the edges.
* Use explicit types.
* Preserve source provenance.
* Preserve stable IDs.
* Add migrations for schema changes.
* Add tests for important behavior.

### 4. Verify

Before finishing:

* Run available checks.
* Run relevant tests.
* Check import/export impact.
* Check rollback or recovery impact.
* Check agent action auditability when relevant.
* Check that generated or derived content remains distinguishable.

### 5. Report

When done, report:

* what changed
* why it changed
* what files changed
* what tests ran
* what was not changed
* any risks or follow-up issues inside the current scope

## Agent Editing Rules

Agents should make the smallest correct change.

Preferred edit order:

1. Edit a field.
2. Edit a block.
3. Edit a note section.
4. Edit a whole note.
5. Edit multiple objects.
6. Change schema.
7. Change architecture.

Broad changes need broad justification.

Agents should preserve naming consistency across code, UI, docs, and data models.

Agents should update documentation when changing:

* domain model
* storage schema
* ingestion states
* agent operations
* search behavior
* view model
* import/export behavior

## Data Integrity Rules

Durable data should be safe, inspectable, and recoverable.

Requirements:

* All durable objects need stable IDs.
* All schema changes need migrations.
* Raw sources should be preserved.
* Derived artifacts should reference their source.
* Generated content should be labeled.
* Review state should be explicit.
* Destructive operations need a recovery path.
* Agent changes need audit records.
* Export should preserve meaningful structure.

Important distinction:

Generated summaries are not raw facts. They are derived artifacts linked to sources.

## Agent Action Rules

Agent changes should be represented as operations.

Every meaningful agent operation should record:

* actor
* operation
* target
* before state or diff
* after state or proposed patch
* rationale
* timestamp
* status
* rollback reference

Default behavior:

* Low-risk internal operations may apply directly.
* User-facing knowledge changes should normally be proposed.
* Entity merges should be proposed.
* Link suggestions should be proposed unless user-approved rules exist.
* Summary generation should produce proposed derived content.
* Deletion should require a recovery path.

## Search and Retrieval Rules

Retrieval should be explicit about content type and trust state.

Search results should show:

* title
* object type
* matching content
* source reference when available
* review state
* generated/user-authored/raw distinction
* relevant entities
* relevant dates

Agent context retrieval should prefer cited, structured context over raw bulk text.

Retrieval should avoid mixing reviewed user knowledge and unreviewed generated content without labeling the distinction.

## Ingestion Rules

Ingestion should preserve source material and produce reviewable structure.

For each source:

* store raw content or file reference
* extract metadata
* parse readable content
* create derived cleaned content when useful
* index searchable content
* propose summary
* propose entities
* propose links
* route to review queue
* promote accepted material into durable knowledge

Failed ingestion should preserve diagnostics and allow retry.

## UI Rules

The UI should be minimalist, readable, and information-first.

Primary surfaces:

* capture
* review queue
* search
* note editor
* source viewer
* entity page
* project dashboard
* timeline
* dossier
* agent change review
* settings

UI behavior:

* Make capture fast.
* Make review fast.
* Make provenance visible.
* Make generated content distinguishable.
* Make important relationships navigable.
* Make views presentable by default.
* Keep labels clear and non-jargony.
* Prefer direct manipulation over hidden configuration.

## Coding Standards

Use Rust with clear boundaries and explicit types.

Rules:

* Prefer explicit domain types over generic maps.
* Prefer Result with meaningful errors.
* Keep parsing, ranking, rendering, and transformation logic testable.
* Keep IO at module boundaries.
* Avoid panics in application logic.
* Avoid global mutable state.
* Add tests for migrations, parsing, import/export, search, and agent operations.
* Keep modules small enough for cold agents to understand.
* Avoid heavy dependencies unless they support the current phase.

## Dependency Policy

Before adding a dependency, check:

* Does it support the current phase?
* Does it work locally?
* Is it maintained?
* Does it preserve data ownership?
* Does it complicate packaging?
* Does it introduce provider lock-in?
* Can it be replaced later without rewriting core data?
* Does it align with the product architecture?

Dependencies should serve the product model, not define it.

## Documentation Standards

Keep architectural decisions visible.

For major decisions, add a short architecture note with:

* decision
* context
* alternatives considered
* consequences
* migration impact

Update docs when changing:

* object model
* schema
* API contracts
* ingestion states
* agent operations
* view rendering
* export/import behavior

Cold agents should be able to recover product intent from the repository, not from chat history.

## Naming Standards

Use consistent product language.

Preferred terms:

* source
* note
* block
* entity
* typed link
* view
* review queue
* agent action
* provenance
* ingestion
* promotion
* rollback
* retrieval
* presentation

Names should reflect the product model. Renaming core concepts is an architecture change.

## Review Checklist

Before completing a change, answer:

1. Which product phase does this support?
2. Which core object does this affect?
3. Does this preserve local-first ownership?
4. Does this preserve structured data?
5. Does this preserve source provenance?
6. Does this keep generated content distinguishable?
7. Does this keep agent changes auditable?
8. Does this keep user data exportable?
9. Did this avoid unrelated refactors?
10. Did tests or checks run?
11. Does documentation need an update?

## Cold Agent Summary

This is a local-first knowledge workbench designed for humans and LLM agents.

Its core value is structured, provenance-aware personal knowledge that can be captured, reviewed, retrieved, edited safely, and presented cleanly.

The main product loop is:

capture → preserve source → structure → review → promote → retrieve → present → safely update

When making changes, protect:

* local ownership
* structured objects
* source provenance
* reviewable ingestion
* typed agent operations
* auditability
* exportability
* clean presentation
* fast retrieval

Make small, coherent changes that strengthen the whole system.
