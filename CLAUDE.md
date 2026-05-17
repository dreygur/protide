# Protide - Development Context

## Project Overview
Native desktop API testing tool built with Rust + GPUI (Zed's GPU-accelerated UI framework).
Supports HTTP, GraphQL, WebSocket, and gRPC protocols.

## Current State (May 2025)
**Progress: ~95% of full plan (Phases 1-6, 8-14 complete)**

### Completed Features
**Core HTTP Client**
- Full HTTP client with GET/POST/PUT/PATCH/DELETE
- URL input with method selector dropdown
- Headers editor (key-value pairs with enable/disable)
- Query params editor (auto-syncs with URL)
- Body editor (JSON, Raw, Form types with file uploads)
- Authentication (Bearer, Basic, API Key in header/query)
- Response viewer with JSON syntax highlighting
- Request timing and size metrics

**Protocol Support**
- GraphQL mode with query/variables editors and schema introspection
- WebSocket mode with connect/disconnect, message sending, and message history
- gRPC mode with proto file loading, service/method selection, metadata, and all streaming modes
- tRPC mode with procedure selection and params editor
- Socket.IO mode (EIO4 handshake, event send/receive, namespace support)
- Mode toggle (HTTP/GraphQL/WS/gRPC/tRPC/Socket.IO)

**Collections & Storage**
- File-based collections (folders = collections, .http files = requests)
- Environment variables with substitution ({{variable}})
- Request history panel
- Save request to .http file

**Scripting & Testing**
- JavaScript engine (rquickjs) for pre/post-request scripts with sandbox interrupt handler
- Test assertions with expect() API
- Script errors surface to console panel (pre and post)

**Import/Export**
- cURL command import
- Postman Collection import
- OpenAPI / Swagger import
- Bruno .bru file import
- Markdown documentation export

**Request Chaining**
- JSONPath extraction from responses
- Variable setting via @set annotations

**Code Generation**
- Generate cURL, Python, JavaScript, Go, Rust code

**Mock Server**
- Local HTTP server for mocking responses
- Route configuration UI

**Tooling & Integrations**
- MCP server (`protide-mcp`) for AI agent integration
- LSP server (`protide-lsp`) for .http file language support
- VS Code extension (`extensions/vscode/`)
- P2P workspace sync (libp2p + CRDT + PAKE auth, behind `full-sync` feature)

**UI/UX**
- System theme support (light/dark)
- Ubuntu Mono font
- Script console output panel
- P2P presence panel

### Project Structure
```
protide/                        # Workspace root
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── http-parser/            # .http file parser (reusable lib)
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── ast.rs          # AST types
│   │       ├── lexer.rs        # Tokenizer
│   │       └── parser.rs       # Parser
│   ├── protide-core/           # All business logic (no UI deps)
│   │   └── src/
│   │       ├── chaining/       # JSONPath @set variable extraction
│   │       ├── codegen/        # Code gen: curl, python, js, go, rust
│   │       ├── execution/      # Protocol runners (http.rs, ws.rs, sio.rs)
│   │       ├── export/         # Markdown doc export
│   │       ├── import/         # cURL, Postman, OpenAPI, Bruno importers
│   │       ├── mock_server/    # Local mock HTTP server (axum)
│   │       ├── models/         # Request, environment models
│   │       ├── protocols/      # gRPC, tRPC protocol handlers
│   │       ├── scripting/      # JS engine (rquickjs) with sandbox + interrupt
│   │       ├── sync/           # P2P sync (libp2p, CRDT, PAKE auth)
│   │       └── workspace/      # Workspace file scanning & management
│   ├── protide-ui/             # GPUI UI layer (no reqwest/tokio-tungstenite)
│   │   └── src/ui/
│   │       ├── main_window.rs  # Top-level layout
│   │       ├── components/     # text_input, code_editor, action_row, etc.
│   │       └── panels/
│   │           ├── request/    # ~40 files: url bar, kv editor, body, auth,
│   │           │               # scripting, code gen, execution glue per protocol
│   │           ├── response/   # Response viewer with JSON highlighting
│   │           ├── explorer/   # File tree + environments (virtualized, ~15 files)
│   │           ├── history.rs  # Request history
│   │           ├── console.rs  # Script console output
│   │           ├── mock_server.rs
│   │           └── presence.rs # P2P presence UI
│   ├── protide/                # Binary entry point (main.rs only)
│   ├── protide-mcp/            # MCP server for AI agent integration
│   └── protide-lsp/            # Language server for .http files
├── extensions/vscode/          # VS Code extension
└── e2e/                        # End-to-end test fixtures per protocol
```

### Key Technical Decisions
1. **HTTP requests**: Using `reqwest::blocking::Client` in background thread (not async) because GPUI doesn't play well with tokio async in UI code
2. **File format**: Extended .http file format with annotations (`# @name`, `# @protocol`, etc.)
3. **No database**: File-system based storage (collections = folders)
4. **UI framework**: GPUI from Zed - GPU-accelerated, immediate mode style

### GPUI Reference
- **Zed editor is the authoritative GPUI example source.** GPUI was invented and built by Zed. Always look at Zed's source code for correct GPUI patterns, event handling, rendering APIs, and idioms before guessing or inventing approaches.
- Zed source: `~/.cargo/git/checkouts/zed-a70e2ad075855582/db5a9be/crates/`
- **Rule: Before writing or fixing any GPUI layout/UI code, search Zed's source first.** Never guess, never trial-and-error. Find the canonical pattern in Zed, then apply it.

### GPUI Gotchas
- `overflow_scroll()` requires `.id()` on the element — but prefer custom scroll with `on_scroll_wheel` + viewport virtualization for large content (avoid rendering thousands of nodes)
- `overflow_scroll()` must have explicit dimensions (`w_full()` + `flex_1()`, or `size_full()`) — without `w_full()`, percentage-based child widths don't resolve to the panel width, breaking `ml_auto()`, `w_full()` on children, and `absolute().right_0()` alignment
- No `overflow_y_scroll()` or `overflow_x_scroll()` - only `overflow_scroll()`
- Theme colors accessed via `theme::current(cx).colors.*`
- Method colors: `theme.method_color("GET")` returns colored Hsla
- `ScrollWheelEvent` / `on_scroll_wheel` available in `gpui::interactive`
- Render one div per logical token/span — never one div per character (massive layout cost)

### Running the App
```bash
cargo run -p protide --release   # Release build recommended for performance
cargo test                       # 190 tests total
```

## Coding Rules

- **Minimum code**: Write the least code that correctly solves the problem. No extra abstraction, no speculative generality, no padding.
- **DRY**: Never write the same logic twice. Extract shared logic into functions, constants, or type aliases immediately — don't wait for a third occurrence.
- **Reuse first**: Before writing anything new, look for an existing function, constant, or component that already does it. Prefer extending what exists over adding new things.
- **No dead code**: Remove unused functions, fields, imports, and variables. Don't leave things "just in case."

## Remaining Phases

### Phase 6: gRPC Support
- ✅ Proto file loading and parsing (protox + prost-reflect)
- ✅ Service/method selection UI
- ✅ Metadata editor
- ✅ Unary execution (reqwest blocking)
- ✅ Server streaming (async reqwest)
- ✅ Client streaming
- ✅ Bidirectional streaming
- ✅ Streaming type detection & UI badge

### Phase 7: tRPC Support
- ✅ Endpoint configuration
- ✅ Query/mutation procedures
- ✅ tRPC v11 native batch support (POST {base}/{p1},{p2}?batch=1)

### Phase 13: API Documentation
- ✅ Markdown export
- Interactive explorer (not started)

### Phase 14: Language Server (LSP)
- ✅ LSP server (`protide-lsp`)
- ✅ VS Code extension (`extensions/vscode/`)
- Zed extension (not started)

### Future Enhancements
- OAuth 2.0 flow UI
- mTLS configuration UI
- Batch collection runner
- Response diffing
- Mock server record/proxy mode
