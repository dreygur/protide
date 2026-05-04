# Protide - Development Context

## Project Overview
Native desktop API testing tool built with Rust + GPUI (Zed's GPU-accelerated UI framework).
Supports HTTP, GraphQL, WebSocket, and gRPC protocols.

## Current State (Jan 2025)
**Progress: ~90% of full plan (Phases 1-6, 8-12 complete)**

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
- GraphQL mode with query/variables editors and syntax highlighting
- WebSocket mode with connect/disconnect, message sending, and message history
- gRPC mode with proto file loading, service/method selection, and metadata
- Mode toggle (HTTP/GraphQL/WS/gRPC)

**Collections & Storage**
- File-based collections (folders = collections, .http files = requests)
- Environment variables with substitution ({{variable}})
- Request history panel
- Save request to .http file

**Scripting & Testing (Phase 8)**
- JavaScript engine (rquickjs) for pre/post-request scripts
- Test assertions with expect() API

**Import/Export (Phase 10)**
- cURL command import
- Postman Collection import

**Request Chaining (Phase 11)**
- JSONPath extraction from responses
- Variable setting via @set annotations

**Code Generation (Phase 12)**
- Generate cURL, Python, JavaScript, Go, Rust code

**Mock Server (Phase 9)**
- Local HTTP server for mocking responses
- Route configuration UI

**UI/UX**
- System theme support (light/dark)
- Ubuntu Mono font

### Project Structure
```
api-dash/
├── Cargo.toml                      # Workspace manifest
├── crates/
│   ├── api-dash/                   # Main desktop app
│   │   └── src/
│   │       ├── main.rs             # Entry point
│   │       ├── app.rs              # App state
│   │       ├── theme.rs            # Theme colors
│   │       ├── workspace/mod.rs    # Workspace management
│   │       ├── models/
│   │       │   ├── mod.rs
│   │       │   ├── environment.rs  # Environment variables
│   │       │   └── request.rs      # Request model
│   │       ├── protocols/
│   │       │   ├── mod.rs
│   │       │   └── http.rs         # Async HTTP client (unused, blocking used instead)
│   │       └── ui/
│   │           ├── mod.rs
│   │           ├── main_window.rs  # Main window layout
│   │           ├── components/
│   │           │   ├── mod.rs
│   │           │   └── text_input.rs   # Text input with selection
│   │           └── panels/
│   │               ├── mod.rs
│   │               ├── explorer.rs     # File tree + environments (~1900 lines)
│   │               ├── history.rs      # Request history
│   │               ├── response.rs     # Response viewer (~1200 lines)
│   │               ├── request_types.rs    # Shared types
│   │               ├── request_utils.rs    # URL encode/decode, base64
│   │               └── request/
│   │                   ├── mod.rs      # Core logic (~1500 lines)
│   │                   ├── render.rs   # UI rendering (~1800 lines)
│   │                   └── tests.rs    # Unit tests
│   └── http-parser/                # .http file parser (reusable crate)
│       └── src/
│           ├── lib.rs
│           ├── ast.rs              # AST types
│           ├── lexer.rs            # Tokenizer
│           └── parser.rs           # Parser
```

### Key Technical Decisions
1. **HTTP requests**: Using `reqwest::blocking::Client` in background thread (not async) because GPUI doesn't play well with tokio async in UI code
2. **File format**: Extended .http file format with annotations (`# @name`, `# @protocol`, etc.)
3. **No database**: File-system based storage (collections = folders)
4. **UI framework**: GPUI from Zed - GPU-accelerated, immediate mode style

### GPUI Reference
- **Zed editor is the authoritative GPUI example source.** GPUI was invented and built by Zed. Always look at Zed's source code for correct GPUI patterns, event handling, rendering APIs, and idioms before guessing or inventing approaches.
- Zed source: `~/.cargo/git/checkouts/zed-a70e2ad075855582/db5a9be/crates/`

### GPUI Gotchas
- `overflow_scroll()` requires `.id()` on the element — but prefer custom scroll with `on_scroll_wheel` + viewport virtualization for large content (avoid rendering thousands of nodes)
- No `overflow_y_scroll()` or `overflow_x_scroll()` - only `overflow_scroll()`
- Theme colors accessed via `theme::current(cx).colors.*`
- Method colors: `theme.method_color("GET")` returns colored Hsla
- `ScrollWheelEvent` / `on_scroll_wheel` available in `gpui::interactive`
- Render one div per logical token/span — never one div per character (massive layout cost)

### Running the App
```bash
cargo run --release   # Release build recommended for performance
cargo test            # 146 tests total
```

## Coding Rules

- **DRY**: Never write the same line of code twice. Extract shared logic into functions, constants, or type aliases.

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
- Endpoint configuration
- Query/mutation procedures

### Phase 13: API Documentation
- Markdown/HTML export
- Interactive explorer

### Phase 14: Language Server (LSP)
- Syntax highlighting for .http files
- Autocomplete
- VS Code/Zed extensions

### Future Enhancements
- Socket.IO support (extend WebSocket mode)
- Bruno .bru file import
- OpenAPI/Swagger import
- Mock server record/proxy mode
