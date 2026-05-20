# Protide - Development Context

## Project Overview
Native desktop API testing tool built with Rust + GPUI (Zed's GPU-accelerated UI framework).
Supports HTTP, GraphQL, WebSocket, gRPC, tRPC, and Socket.IO protocols.

## Current State (May 2026)
**Progress: All original phases complete + extras (P2P collab, MCP server)**

### Completed Features

**Core HTTP Client**
- Full HTTP client with GET/POST/PUT/PATCH/DELETE
- URL input with method selector dropdown
- Headers editor (key-value pairs with enable/disable)
- Query params editor (auto-syncs with URL)
- Body editor (JSON, Raw, Form types with file uploads)
- Authentication (Bearer, Basic, API Key in header/query)
- Response viewer with JSON syntax highlighting + collapsible tree
- Request timing and size metrics

**Protocol Support**
- GraphQL: query/variables editors, syntax highlighting
- WebSocket: connect/disconnect, message sending, history, autoscroll
- Socket.IO: full execution with event support
- gRPC: proto loading, service/method selection, metadata, all streaming types
- tRPC: query/mutation procedures
- Mode toggle across all protocols

**Collections & Storage**
- File-based collections (folders = collections, .http files = requests)
- Environment variables with substitution (`{{variable}}`)
- Request history panel
- Save request to .http file

**Scripting & Testing**
- JavaScript engine (rquickjs) for pre/post-request scripts
- Test assertions with `expect()` API

**Import/Export**
- cURL command import
- Postman Collection import
- Bruno .bru file import
- OpenAPI/Swagger import
- Markdown export (`protide-core/src/export/markdown.rs`)

**Request Chaining**
- JSONPath extraction from responses
- Variable setting via `@set` annotations

**Code Generation**
- cURL, Python, JavaScript, Go, Rust

**Mock Server**
- Local HTTP server for mocking responses
- Route configuration UI
- Record/proxy mode: forwards requests to target, captures responses as static routes

**Collaboration (Local-First Sync)**
- CRDT-based state (LWW registers, Lamport timestamps)
- P2P via libp2p (mDNS + Gossipsub)
- BYOB file sync (Dropbox/Drive/GitHub)
- UDP live probe for LAN presence
- PAKE secure pairing
- Presence panel UI

**Tooling**
- LSP server (`protide-lsp`): hover, completion, semantic tokens for .http files
- MCP server (`protide-mcp`): JSON-RPC 2.0 over stdio, exposes `send_request` tool
- Console panel: structured log bus

**UI/UX**
- System theme support (light/dark)
- Ubuntu Mono font
- JSON tree with drag-select
- ActionRow component with scroll-safe hover-revealed actions

### Project Structure
```
protide/
├── Cargo.toml                          # Workspace manifest
├── crates/
│   ├── protide/                        # Binary entry point (main.rs only)
│   ├── protide-ui/                     # All GPUI UI code
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── theme.rs
│   │       ├── prefs.rs
│   │       ├── session.rs
│   │       ├── last_paths.rs
│   │       └── ui/
│   │           ├── main_window/        # Main window layout (split into 9 files)
│   │           ├── components/
│   │           │   ├── action_row.rs
│   │           │   ├── code_editor/    # Syntax-highlighted editor
│   │           │   ├── icons.rs
│   │           │   ├── modal.rs
│   │           │   ├── selectable_text.rs
│   │           │   ├── text_input.rs
│   │           │   └── ui_helpers.rs
│   │           └── panels/
│   │               ├── console.rs      # Log bus panel
│   │               ├── docs/           # API documentation viewer
│   │               ├── explorer/       # File tree + environments (split into ~15 files)
│   │               ├── history.rs
│   │               ├── mock_server/    # Mock server panel (split into 3 files)
│   │               ├── presence.rs     # Collaboration presence UI
│   │               ├── request/        # Request panel (split into ~40 files)
│   │               ├── request_types.rs
│   │               ├── request_utils.rs
│   │               └── response/       # Response panel (split into ~12 files)
│   ├── protide-core/                   # Business logic (no UI)
│   │   └── src/
│   │       ├── chaining/               # JSONPath extraction, @set
│   │       ├── codegen/                # curl/python/js/go/rust generators
│   │       ├── execution/              # http, ws, sio executors
│   │       ├── export/                 # Markdown export
│   │       ├── import/                 # curl, postman, bruno, openapi
│   │       ├── mock_server/            # Local HTTP mock server
│   │       ├── models/                 # Request, Environment models
│   │       ├── protocols/              # grpc, trpc protocol logic
│   │       ├── scripting/              # rquickjs JS engine
│   │       ├── sync/                   # CRDT, P2P, PAKE, file sync
│   │       └── workspace/
│   ├── protide-lsp/                    # LSP server for .http files (tower-lsp)
│   ├── protide-mcp/                    # MCP server (JSON-RPC 2.0 over stdio)
│   └── http-parser/                    # .http file parser (reusable crate)
│       └── src/
│           ├── ast.rs
│           ├── lexer.rs
│           └── parser.rs
```

### Key Technical Decisions
1. **HTTP requests**: `reqwest::blocking::Client` in background thread - GPUI doesn't play well with tokio async in UI code
2. **File format**: Extended .http file format with annotations (`# @name`, `# @protocol`, etc.)
3. **No database**: File-system based storage (collections = folders)
4. **UI framework**: GPUI from Zed - GPU-accelerated, immediate mode style
5. **Collaboration**: Local-first CRDT, no central server required
6. **MCP**: JSON-RPC 2.0 over stdio - lets AI tools (Claude, etc.) drive requests

### GPUI Reference
- **Zed editor is the authoritative GPUI example source.** Always look at Zed's source code for correct GPUI patterns before guessing.
- Zed source: `~/.cargo/git/checkouts/zed-a70e2ad075855582/db5a9be/crates/`
- **Rule: Before writing or fixing any GPUI layout/UI code, search Zed's source first.**

### GPUI Gotchas
- `overflow_scroll()` requires `.id()` on the element
- `overflow_scroll()` must have explicit dimensions (`w_full()` + `flex_1()`, or `size_full()`) - without `w_full()`, percentage-based child widths don't resolve, breaking `ml_auto()`, `w_full()` on children, and `absolute().right_0()` alignment
- No `overflow_y_scroll()` or `overflow_x_scroll()` - only `overflow_scroll()`
- Theme colors: `theme::current(cx).colors.*`
- Method colors: `theme.method_color("GET")` returns `Hsla`
- `ScrollWheelEvent` / `on_scroll_wheel` in `gpui::interactive`
- Render one div per logical token/span - never one div per character (massive layout cost)
- Spacer divs cause dark rendering artifacts - avoid them

### Running the App
```bash
cargo run --release   # Release build recommended
cargo test            # ~190 tests total (19 http-parser + 93 protide-core + 78 protide-ui)
```

## Coding Rules

- **Minimum code**: Write the least code that correctly solves the problem. No extra abstraction, no speculative generality, no padding.
- **DRY**: Never write the same logic twice. Extract shared logic into functions, constants, or type aliases immediately.
- **Reuse first**: Before writing anything new, look for an existing function, constant, or component that already does it.
- **No dead code**: Remove unused functions, fields, imports, and variables.
- **File size**: Max 333 lines per file (tests excluded). Split before exceeding.

## Remaining / Future Work

- VS Code / Zed extension packaging for LSP
- Bruno import completeness (verify edge cases)
- OpenAPI import completeness
- Socket.IO: advanced namespaces/rooms UI
