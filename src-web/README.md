# PKM Workbench UI

React + TypeScript + Vite frontend for the Personal Knowledge Management system.

## Stack

- **Vite** - Lightning-fast build tool
- **React 18** - UI framework
- **TypeScript** - Type safety
- **Tailwind CSS** - Utility-first styling
- **Floating UI** - Positioning for popovers and tooltips
- **Lucide React** - Icon library (when needed)
- **Tauri API** - Bridge to Rust backend

## Setup

```bash
npm install
```

## Development

```bash
npm run dev
```

This starts the Vite dev server. When running through Tauri (`cargo tauri dev`), this will be automatically invoked.

## Build

```bash
npm run build
```

Outputs to `../crates/pkm-app/dist/` for Tauri to package.

## Features

### Priority 1: Hover Previews ✅
- Entity links with inline preview cards
- Floating UI positioning
- Tauri IPC integration

### Priority 2: Argument Trees
- React Flow for visualization
- Dagre.js for hierarchical layout
- Colored edges by semantic meaning

### Priority 3: Progressive Disclosure Graphs
- Double-click to expand nodes
- Animated node spawning
- Force-directed physics
