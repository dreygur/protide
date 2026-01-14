# API Dash - Development Context

## Project Overview
Native desktop API testing tool built with Rust + GPUI (Zed's GPU-accelerated UI framework).
Supports HTTP with plans for GraphQL, gRPC, WebSocket, Socket.IO, and tRPC.

## Current State (Jan 2025)
**Progress: ~25% of full plan (Phases 1-2 complete, Phase 4 partial)**

### Completed Features
- Full HTTP client with GET/POST/PUT/PATCH/DELETE
- URL input with method selector dropdown
- Headers editor (key-value pairs with enable/disable)
- Query params editor (auto-syncs with URL)
- Body editor (JSON, Raw, Form types)
- Authentication (Bearer, Basic, API Key in header/query)
- Response viewer with JSON syntax highlighting
- Request timing and size metrics
- File-based collections (folders = collections, .http files = requests)
- Environment variables with substitution ({{variable}})
- Request history panel
- Save request to .http file
- System theme support (light/dark)

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

### GPUI Gotchas
- `overflow_scroll()` requires `.id()` on the element
- No `overflow_y_scroll()` or `overflow_x_scroll()` - only `overflow_scroll()`
- Theme colors accessed via `theme::current(cx).colors.*`
- Method colors: `theme.method_color("GET")` returns colored Hsla

### Running the App
```bash
cargo run --release   # Release build recommended for performance
cargo test            # 78 tests total
```

## Remaining Phases (from plan)

### Phase 3: GraphQL Support
- GraphQL query editor
- Variables editor
- Schema introspection

### Phase 5: WebSocket & Socket.IO
- Connection UI, message panel
- tokio-tungstenite for WebSocket
- rust-socketio for Socket.IO

### Phase 6: gRPC Support
- Proto file loading (tonic/prost)
- Service/method selection
- Streaming support

### Phase 7: tRPC Support
- Endpoint configuration
- Query/mutation procedures

### Phase 8: Scripting & Testing
- JavaScript engine (rquickjs or boa)
- Pre/post request scripts
- Test assertions with expect() API

### Phase 9: Mock Server
- Local HTTP server for mocking
- Record mode (proxy)

### Phase 10: Import/Export
- Postman Collection import
- cURL command import
- Bruno .bru import
- OpenAPI/Swagger import

### Phase 11: Request Chaining
- JSONPath extraction
- Variable setting from responses
- Dependency ordering

### Phase 12: Code Generation
- Generate Python, JS, Go, Rust, cURL

### Phase 13: API Documentation
- Markdown/HTML export
- Interactive explorer

### Phase 14: Language Server (LSP)
- Syntax highlighting
- Autocomplete
- VS Code/Zed extensions

## Full Plan Reference
See `/home/rakib/.claude/plans/golden-swinging-porcupine.md` for complete specification.
