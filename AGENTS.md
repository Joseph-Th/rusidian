## Project Identity

This project is a local-first, markdown-backed personal knowledge workbench and data visualization system. 

It helps a user capture information, parse it into a graph of entities and links, and present that information through highly interactive visual views (Graphs, Timelines, Canvases).

## North Star

The system should feel like a calm, intelligent workspace. It should seamlessly bridge the gap between simple text files (Markdown) and rich, interactive visual intelligence. 

The product should help the user visually answer:
* How are these ideas connected? (Graph View)
* When did these events happen? (Timeline View)
* How can I spatially organize this research? (Canvas View)

## Core Product Principles

### 1. Local-first, File-backed Ownership
User data lives entirely on the local file system. 
Notes and Sources are stored as standard `.md` files. Metadata (Links, Entities, Canvas layouts) are stored as simple `.json` files. The system must always be able to rebuild its state from these files (`pkm-fs`). 

### 2. Structured Knowledge as a Graph
The system stores knowledge as a graph of structured objects. 
Markdown is the editing format, but internally, the system maps out typed relationships (`Links`) between `Notes`, `Sources`, and extracted `Entities`. This graph is what powers the visualizations.

### 3. Presentation-First Visualization
The primary output of this application is not just text, but visual clarity.
The system provides interactive views (Graph, Canvas, Timeline) that are generated automatically from the underlying graph data.

### 4. AI as an Accelerator, Not a Manager
AI (LLMs) are used to do the heavy lifting: extracting text from URLs (via Jina), summarizing content, and inferring links between entities. However, we trust the user. We do not need heavy, restrictive "Agent Action" rollback systems to guard against the AI. Keep AI operations simple and direct.

### 5. Opinionated Simplicity
Prefer working features over infinite abstraction. 
* We use a single in-memory `RwLock<VaultState>` because it is fast enough for personal use.
* We do not use PostgreSQL, SQLite, or ORMs.
* We rely on Tauri commands to pass clean, formatted JSON to the frontend.

## Core Domain Model

### Source
A raw piece of captured information (e.g., a web article fetched via URL). Stored as Markdown. It is the bedrock from which notes and entities are derived.

### Note & Block
A durable knowledge object containing user or AI-generated text. Stored as Markdown. A Note consists of Blocks (paragraphs, tables, embeds). Block IDs are preserved in Markdown via HTML comments (`<!-- block:uuid -->`).

### Entity
A normalized concept (Person, Organization, Topic) extracted from Sources or Notes. Stored in `entities.json`.

### Typed Link
A directed edge connecting any two objects (`Source`, `Note`, `Block`, `Entity`). Links have types (e.g., `Supports`, `Mentions`, `DerivedFrom`). Stored in `links.json`. This is the engine of the Graph View.

### View
A saved layout or filter for data visualization. 
Currently supported Views:
* `GraphView` (Nodes and Links)
* `CanvasView` (Spatial organization)
* `Timeline` (Chronological ordering)
* `ReadingQueue` / `ReviewQueue` (Lists)

## Product Architecture

**1. `pkm-core` (Types):** Pure Rust structs and enums. No I/O.
**2. `pkm-fs` (Storage):** The *only* database. Reads `.md` and `.json` files into an in-memory `HashMap`. Fast, simple, synchronous.
**3. `pkm-app` (Backend/Service):** Provides the Tauri commands and core logic (e.g., calculating graph neighbors, running the Jina URL fetcher).
**4. Frontend (UI):** A Tauri-powered web frontend (React/Svelte/etc.) responsible *only* for rendering the data provided by `pkm-app` into visual components (React Flow, Force Graphs, etc.).

## Development Phases

### Phase 1: Backend Foundation (COMPLETE)
* Local Markdown/JSON storage (`pkm-fs`).
* Note/Block parsing.
* Core data types.

### Phase 2: Frontend Scaffolding (CURRENT)
* Initialize the Tauri frontend framework (React/Svelte).
* Connect basic UI to `list_notes` and `get_note` commands.
* Build a basic Markdown text editor.

### Phase 3: Visualizations
* Implement the Graph library in the frontend.
* Wire up `get_link_network` to render interactive node/edge graphs.
* Build the Timeline and Canvas views.

### Phase 4: AI Ingestion & Enhancement
* Connect the frontend to the `ingest_bulk_links` command.
* Add UI for pasting URLs and watching them appear in the Graph.

## Agent Workflow (Strict Rules)

Every coding agent must follow this workflow to prevent codebase collapse.

### 1. Orient & Restrict
* Identify the requested task.
* **Is this adding a new architectural layer?** If yes, STOP. Ask the user first.
* **Does this require SQL or a new database?** If yes, STOP. Use `pkm-fs`.

### 2. Implement (Keep it Flat)
* Prefer small changes.
* Do not create stubs for features the user hasn't explicitly asked for.
* If modifying the UI, write clean, modern, component-based code.
* If modifying Rust, ensure you are passing flat, easily serializable structs to Tauri.

### 3. Verify
* Run `cargo test`.
* Ensure zero warnings in Rust.
* Ensure no new ghost features were accidentally created.

## Coding Standards

* **Rust:** Keep IO at the edges. Use explicit types. Do not hold `RwLock` across await points. 
* **Tauri:** Commands in `main.rs` should contain no business logic; they simply delegate to `service.rs` or `graph.rs`.
* **Frontend:** Keep state management simple. The source of truth is the Rust backend; the frontend is a visual projection of that state.
* **Dependencies:** Do not add heavy dependencies unless absolutely necessary. 

## Final Cold Agent Summary

This is a **simple, local-first data visualization tool**. It parses Markdown files and visualizes the connections between them as graphs and timelines. Do not over-engineer it. Write clean, direct code that gets data from the file system onto the user's screen as beautifully and simply as possible.