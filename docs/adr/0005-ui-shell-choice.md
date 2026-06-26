# ADR 0005 — UI shell choice: Tauri

Status: accepted · Date: 2026-06-25

## Decision

Adopt **Tauri** as the UI shell for the application's desktop presence.

Tauri is a lightweight, Rust-backed desktop framework that:
- Runs a web frontend (HTML/CSS/JavaScript or TypeScript/React) in a lightweight Chromium wrapper.
- Embeds the Rust backend directly; commands are exposed via RPC.
- Provides native window, menu, and file system integration.
- Has a small binary footprint (~10MB vs 100MB+ for Electron).
- Keeps the backend and frontend in the same repository and build process.

## Context

ADR 0001 deferred the UI shell choice. The foundation crates (`pkm-core`, `pkm-storage`, `pkm-search`, `pkm-agent`, `pkm-app`) are now stable enough to add a UI layer.

Tauri was chosen because:
1. **Rust alignment.** The entire backend is Rust; Tauri lets the frontend consume
   it via typed RPC without FFI overhead or serialization round-trips.
2. **Local-first simplicity.** Tauri runs on the user's machine with no cloud
   dependency; data stays local by default.
3. **Lightweight distribution.** Tauri binaries are smaller and faster to
   download than Electron.
4. **Mature ecosystem.** Tauri is actively maintained, has good documentation,
   and is used in production (e.g., Slack's desktop experiments).

## Alternatives considered

1. **Electron.** Rejected: Heavier binary (~200MB for a minimal app), separate
   Node.js runtime, and no direct Rust integration. Overkill for a single-user
   local app.
2. **GTK / Qt (native Rust bindings).** Rejected: Excellent for system
   performance, but less flexible UI design freedom; harder to iterate on
   layouts without recompiling. Web frontend is more accessible to designers.
3. **Web-only (hosting in the browser).** Rejected: Violates the local-first
   principle (ADR 0001); adds cloud dependency and third-party control.
4. **No UI (CLI/TUI only).** Rejected: Presentation-first views (F-series) require
   visual layout and rich rendering that a terminal cannot deliver well.

## Consequences

1. **Frontend layer.** The `pkm-app` crate adds a Tauri binary and frontend
   (HTML/CSS/JavaScript or TypeScript framework). The frontend calls Rust
   commands via `invoke()`.
2. **Command API.** All user-facing operations are exposed as Tauri commands
   wired to `AppService`. Commands are the boundary between UI and business
   logic.
3. **Build process.** `cargo build` or `tauri dev` builds both backend and
   frontend. Distribution creates a single executable.
4. **Cross-platform.** Tauri supports macOS, Windows, and Linux (with some
   additional setup). Development can target any of these.
5. **Future flexibility.** If the UI needs to scale beyond Tauri (e.g., web
   version, mobile companion), the backend remains separate and reusable;
   only the frontend swaps out.
