# API Dash - API Testing Tool

A **free and open-source** native desktop API testing application built with Rust + GPUI, supporting HTTP, GraphQL, gRPC, RPC, tRPC, WebSocket, and Socket.IO.

## Philosophy

- **Open file format**: Uses `.http` files (extended) - human-readable, git-friendly, interoperable
- **File-system based**: Collections = folders, Requests = files. No proprietary database.
- **Portable**: Your API collections are just files you can version control, share, and edit anywhere

## Tech Stack

- **UI Framework**: GPUI (Zed's GPU-accelerated UI framework)
- **UI Components**: gpui-component (Longbridge)
- **HTTP Client**: reqwest (async)
- **GraphQL**: graphql_client
- **gRPC**: tonic
- **WebSocket**: tokio-tungstenite
- **Socket.IO**: rust-socketio
- **Serialization**: serde, serde_json
- **Storage**: File-system based (.http files + folders)

---

## File Format Specification

### Workspace Structure
```
my-api-project/                 # Workspace root (a folder user opens)
├── .api-dash.json              # Workspace settings (optional)
├── environments/
│   ├── dev.env.json
│   └── prod.env.json
├── users/                      # Collection folder
│   ├── get-users.http
│   ├── create-user.http
│   └── auth/                   # Nested collection
│       └── login.http
└── orders/
    ├── list-orders.http
    └── websocket-updates.http
```

### .http File Format (Extended)

**Standard HTTP Request:**
```http
### Get all users
# @name get-users
# @description Fetches paginated user list

GET https://api.example.com/users?page=1
Authorization: Bearer {{access_token}}
Content-Type: application/json
```

**GraphQL Request:**
```http
### Get user by ID
# @protocol graphql

POST https://api.example.com/graphql
Content-Type: application/json
X-GraphQL-Operation: query

{
  "query": "query GetUser($id: ID!) { user(id: $id) { name email } }",
  "variables": { "id": "123" }
}
```

**WebSocket:**
```http
### Live updates
# @protocol websocket

WEBSOCKET wss://api.example.com/live
Authorization: Bearer {{token}}

---messages---
{"subscribe": "orders"}
{"subscribe": "notifications"}
```

**gRPC:**
```http
### Get user profile
# @protocol grpc
# @proto ./protos/user.proto

GRPC grpc://localhost:50051/user.UserService/GetProfile
grpc-metadata-authorization: Bearer {{token}}

{
  "user_id": "123"
}
```

### Environment Files (.env.json)
```json
{
  "name": "Development",
  "variables": {
    "base_url": "https://dev.api.example.com",
    "access_token": "dev-token-xxx"
  }
}
```

### Scripts in .http Files
```http
### Create user with auth
# @name create-user

# @pre-script
// JavaScript runs before request
const timestamp = Date.now();
request.headers['X-Timestamp'] = timestamp;
request.headers['Authorization'] = 'Bearer ' + env.access_token;

POST {{base_url}}/users
Content-Type: application/json

{"name": "John", "email": "john@example.com"}

# @post-script
// JavaScript runs after response
const userId = response.body.id;
env.set('last_user_id', userId);

# @tests
// Assertions
expect(response.status).toBe(201);
expect(response.body).toHaveProperty('id');
expect(response.time).toBeLessThan(1000);
```

### Request Chaining
```http
### Login to get token
# @name login

POST {{base_url}}/auth/login
Content-Type: application/json

{"email": "user@example.com", "password": "secret"}

# @set access_token = $.token
# @set user_id = $.user.id

###

### Get user profile (depends on login)
# @name get-profile
# @depends login

GET {{base_url}}/users/{{user_id}}
Authorization: Bearer {{access_token}}
```

### Mock Definition (.mock.json)
```json
{
  "mocks": [
    {
      "request": {
        "method": "GET",
        "path": "/users/:id"
      },
      "response": {
        "status": 200,
        "headers": {"Content-Type": "application/json"},
        "body": {"id": "{{params.id}}", "name": "Mock User"}
      }
    }
  ]
}
```

---

## Project Structure

```
api-dash/
├── Cargo.toml                  # Workspace manifest
├── crates/
│   ├── api-dash/               # Main desktop app
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── http-parser/            # .http file parser (reusable)
│   │   ├── Cargo.toml
│   │   └── src/
│   ├── http-lsp/               # Language Server
│   │   ├── Cargo.toml
│   │   └── src/
│   └── http-cli/               # CLI tool (optional)
│       ├── Cargo.toml
│       └── src/
├── extensions/
│   ├── vscode/                 # VS Code extension
│   └── zed/                    # Zed extension
└── docs/                       # Documentation

# Main app structure (crates/api-dash/src/):
├── src/
│   ├── main.rs                 # App entry point
│   ├── app.rs                  # App state, global context
│   ├── theme.rs                # Theme (follows system preference)
│   │
│   ├── ui/                     # UI Layer
│   │   ├── mod.rs
│   │   ├── main_window.rs      # Main window with dock system
│   │   ├── dock/               # Dockable panel system
│   │   │   ├── mod.rs
│   │   │   ├── dock_area.rs    # Dock container
│   │   │   ├── panel.rs        # Panel wrapper
│   │   │   └── splitter.rs     # Resizable splits
│   │   ├── panels/             # Individual panels
│   │   │   ├── mod.rs
│   │   │   ├── explorer.rs     # File tree sidebar
│   │   │   ├── request.rs      # Request editor panel
│   │   │   ├── response.rs     # Response viewer panel
│   │   │   └── console.rs      # Output/logs console
│   │   ├── editor/             # Request editors
│   │   │   ├── mod.rs
│   │   │   ├── form_editor.rs  # Form-based request editor
│   │   │   ├── code_editor.rs  # Raw .http code editor
│   │   │   └── key_value.rs    # Key-value pair component
│   │   └── components/         # Shared components
│   │       ├── mod.rs
│   │       ├── method_badge.rs
│   │       ├── status_badge.rs
│   │       ├── url_bar.rs
│   │       └── tabs.rs
│   │
│   ├── protocols/              # Protocol implementations
│   │   ├── mod.rs
│   │   ├── http.rs
│   │   ├── graphql.rs
│   │   ├── grpc.rs
│   │   ├── websocket.rs
│   │   ├── socketio.rs
│   │   └── trpc.rs
│   │
│   ├── models/                 # Data models
│   │   ├── mod.rs
│   │   ├── request.rs
│   │   ├── response.rs
│   │   ├── environment.rs
│   │   └── workspace.rs        # Workspace/collection model
│   │
│   ├── parser/                 # .http file parser
│   │   ├── mod.rs
│   │   ├── lexer.rs
│   │   └── parser.rs
│   │
│   ├── workspace/              # Workspace management
│   │   ├── mod.rs
│   │   ├── file_watcher.rs     # Watch for file changes
│   │   └── loader.rs           # Load/save .http files
│   │
│   ├── scripting/              # JavaScript engine
│   │   ├── mod.rs
│   │   ├── engine.rs           # JS runtime (rquickjs)
│   │   ├── api.rs              # request/response/env APIs
│   │   └── assertions.rs       # expect() test framework
│   │
│   ├── mock/                   # Mock server
│   │   ├── mod.rs
│   │   ├── server.rs           # HTTP mock server
│   │   ├── router.rs           # Request matching
│   │   └── recorder.rs         # Proxy recording mode
│   │
│   ├── import/                 # Import/Export
│   │   ├── mod.rs
│   │   ├── postman.rs          # Postman Collection parser
│   │   ├── curl.rs             # cURL command parser
│   │   ├── bruno.rs            # .bru file parser
│   │   ├── openapi.rs          # OpenAPI/Swagger import
│   │   └── export.rs           # Export to other formats
│   │
│   ├── codegen/                # Code generation
│   │   ├── mod.rs
│   │   ├── python.rs
│   │   ├── javascript.rs
│   │   ├── go.rs
│   │   ├── rust.rs
│   │   └── curl.rs
│   │
│   └── docs/                   # Documentation generation
│       ├── mod.rs
│       ├── markdown.rs         # Markdown export
│       └── html.rs             # Static HTML site
│
└── assets/
    └── icons/
```

---

## Core Architecture

### 1. App State (Global)

```rust
pub struct AppState {
    pub collections: Vec<Collection>,
    pub environments: Vec<Environment>,
    pub active_environment: Option<String>,
    pub open_tabs: Vec<RequestTab>,
    pub active_tab_index: usize,
    pub db: Database,
}
```

### 2. Request Model (Universal)

```rust
pub enum ProtocolType {
    Http,
    GraphQL,
    Grpc,
    WebSocket,
    SocketIO,
    Trpc,
}

pub struct Request {
    pub id: Uuid,
    pub name: String,
    pub protocol: ProtocolType,
    pub url: String,
    pub method: HttpMethod,          // For HTTP/GraphQL
    pub headers: Vec<KeyValue>,
    pub body: RequestBody,
    pub auth: Option<AuthConfig>,
}

pub enum RequestBody {
    None,
    Json(String),
    FormData(Vec<KeyValue>),
    Raw(String),
    GraphQL { query: String, variables: String },
    Grpc { service: String, method: String, message: String },
}
```

### 3. Response Model

```rust
pub struct Response {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValue>,
    pub body: String,
    pub time_ms: u64,
    pub size_bytes: usize,
}
```

---

## MVP Implementation Phases

### Phase 1: Foundation
1. Initialize Cargo workspace with dependencies
2. Set up GPUI app skeleton with main window
3. Implement basic theme and styling
4. Create main layout (sidebar, request panel, response panel)

### Phase 2: HTTP Client
1. Implement HTTP request builder UI
   - URL input with method selector
   - Headers editor (key-value pairs)
   - Body editor (JSON, form-data, raw)
   - Auth options (Bearer, Basic, API Key)
2. Implement reqwest-based HTTP client
3. Response viewer with syntax highlighting
4. Request/response timing and size metrics

### Phase 3: GraphQL Support
1. GraphQL query editor with syntax support
2. Variables editor
3. Schema introspection (optional)
4. Response viewer for GraphQL

### Phase 4: Collections & Persistence
1. SQLite database setup
2. Collection CRUD operations
3. Request history
4. Environment variables with substitution

### Phase 5: WebSocket & Socket.IO
1. WebSocket connection UI
2. Message send/receive panel
3. Connection state management
4. Socket.IO event handling

### Phase 6: gRPC Support
1. Proto file loading
2. Service/method selection
3. Message builder
4. Streaming support (unary, server, client, bidirectional)

### Phase 7: tRPC Support
1. tRPC endpoint configuration
2. Procedure type selection (query/mutation)
3. Input builder

### Phase 8: Scripting & Testing
1. JavaScript engine integration (rquickjs/boa)
2. Pre-request scripts (modify headers, generate tokens, etc.)
3. Post-response scripts (extract values, chain requests)
4. Test assertions with expect() API
5. Test runner with pass/fail reporting

### Phase 9: Mock Server
1. Built-in mock server (local HTTP server)
2. Define mock endpoints from request/response pairs
3. Dynamic responses with JavaScript
4. Record mode (proxy real API, save responses)

### Phase 10: Import/Export
1. **Postman import**: Parse Postman Collection v2.1 JSON, convert to .http files
2. **cURL import**: Parse cURL commands, generate .http files
3. **Bruno import**: Parse .bru files, convert to .http format
4. **OpenAPI import**: Generate requests from OpenAPI/Swagger specs
5. **Export**: Export collections to Postman format for sharing

### Phase 11: Request Chaining
1. Extract values from responses using JSONPath/XPath
2. Store in variables: `# @set userId = $.data.id`
3. Chain requests with dependencies
4. Visual chain builder in UI

### Phase 12: Code Generation
1. Generate client code from .http requests
2. Supported languages: Python, JavaScript/TypeScript, Go, Rust, cURL
3. Template-based generation (customizable)
4. Copy to clipboard or save to file

### Phase 13: API Documentation
1. Generate markdown docs from collections
2. Export to HTML static site
3. Interactive API explorer (like Swagger UI)
4. Sync docs with request changes

### Phase 14: Language Server (LSP)
1. **Standalone LSP binary** for .http files with extended syntax
2. Features:
   - Syntax highlighting
   - Autocomplete (methods, headers, variables)
   - Hover documentation
   - Go-to-definition for variables
   - Error diagnostics
   - Code actions (send request, format)
3. **Editor extensions**:
   - VS Code extension (uses LSP)
   - Zed extension (uses LSP)
   - Neovim config
4. Publish LSP as separate crate for reuse

### Phase 15: OAuth Support
1. OAuth 2.0 flows:
   - Authorization Code (with PKCE)
   - Client Credentials
   - Password Grant
   - Implicit (deprecated but supported)
2. OAuth 1.0a support
3. Token management:
   - Auto-refresh tokens
   - Token storage (secure)
   - Multiple OAuth profiles
4. Built-in OAuth callback server for auth code flow

---

## UI/UX Design

### Design Principles
- **Flexible/Dockable Layout**: IDE-like panel system users can arrange freely
- **Hybrid Editor**: Toggle between form-based UI and raw .http code editor
- **System Theme**: Follow OS light/dark mode preference
- **Minimal Clutter**: Focus on content, reduce visual noise

### Default Layout (Customizable)
```
┌─────────────────────────────────────────────────────────┐
│  [Logo]  API Dash              [Theme] [Layout] [─][□][×]│
├─────────┬───────────────────────────────────────────────┤
│         │  [Tab1] [Tab2] [+]              [Form│Code]   │
│ Explorer ├───────────────────────────────────────────────┤
│         │  [GET ▼] [https://api.example.com/users] [▶]  │
│ ▼ users/ ├───────────────────────────────────────────────┤
│   get.http   │  Params │ Headers │ Body │ Auth            │
│   post.http  │  ┌─────────────────────────────────────┐    │
│ ▼ orders/    │  │ (Form or Code view toggleable)      │    │
│   list.http  │  └─────────────────────────────────────┘    │
│         ├─────────────────────────────────────────────│
│         │  Response           200 OK  150ms  2.3KB    │
│ [Env▼]  │  ┌─────────────────────────────────────┐    │
│         │  │ { "data": [...] }                    │    │
└─────────┴───────────────────────────────────────────────┘
```

### Panel System
- **Dockable Panels**: File explorer, request editor, response viewer, console
- **Split Views**: Horizontal/vertical splits, drag to resize
- **Tabs**: Multiple requests open simultaneously
- **Floating Panels**: Detach any panel into floating window

### Editor Modes
1. **Form Mode** (default for beginners):
   - URL bar with method dropdown
   - Tab sections: Params, Headers, Body, Auth
   - Key-value editors with add/remove buttons

2. **Code Mode** (for power users):
   - Full .http file editor with syntax highlighting
   - Autocomplete for headers, methods, variables
   - Live validation and error highlighting

3. **Toggle Button**: Switch between modes instantly
   - Changes sync bidirectionally

---

## Dependencies (Cargo.toml)

```toml
[package]
name = "api-dash"
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/user/api-dash"

[dependencies]
gpui = { git = "https://github.com/zed-industries/zed", branch = "main" }
gpui-component = { git = "https://github.com/longbridge/gpui-component" }

# Async runtime
tokio = { version = "1", features = ["full"] }

# HTTP
reqwest = { version = "0.12", features = ["json", "rustls-tls"] }

# GraphQL
graphql_client = "0.14"

# gRPC
tonic = "0.12"
prost = "0.13"

# WebSocket
tokio-tungstenite = "0.26"

# Socket.IO
rust_socketio = "0.7"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# File watching (for live reload)
notify = "7"

# Utils
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
anyhow = "1"
thiserror = "2"
dirs = "6"  # For config/data directories
```

---

## Verification

1. **Build**: `cargo build`
2. **Run**: `cargo run`
3. **Test HTTP**: Send GET request to `https://httpbin.org/get`
4. **Test GraphQL**: Query `https://countries.trevorblades.com/graphql`
5. **Test WebSocket**: Connect to `wss://echo.websocket.org`

---

## Files to Create (Phase 1 - Foundation)

### Workspace Setup
1. `Cargo.toml` - Workspace manifest
2. `crates/http-parser/Cargo.toml` - Parser crate manifest
3. `crates/http-parser/src/lib.rs` - Parser library root
4. `crates/http-parser/src/lexer.rs` - .http file tokenizer
5. `crates/http-parser/src/parser.rs` - .http file parser
6. `crates/http-parser/src/ast.rs` - AST types

### Main App
7. `crates/api-dash/Cargo.toml` - App manifest
8. `crates/api-dash/src/main.rs` - Entry point, GPUI init
9. `crates/api-dash/src/app.rs` - Global app state
10. `crates/api-dash/src/theme.rs` - System theme detection
11. `crates/api-dash/src/ui/mod.rs` - UI module
12. `crates/api-dash/src/ui/main_window.rs` - Main window
13. `crates/api-dash/src/ui/panels/explorer.rs` - File tree
14. `crates/api-dash/src/ui/panels/request.rs` - Request panel
15. `crates/api-dash/src/ui/panels/response.rs` - Response panel
16. `crates/api-dash/src/models/mod.rs` - Models
17. `crates/api-dash/src/models/request.rs` - Request types
18. `crates/api-dash/src/protocols/mod.rs` - Protocols
19. `crates/api-dash/src/protocols/http.rs` - HTTP client

---

## Current Sprint: Code Editor Component

### Overview

Create a reusable `CodeEditor` component inspired by Zed's editor architecture for:
- **Body Editor**: Editable JSON with syntax highlighting, line numbers
- **Response Viewer**: Read-only viewer with syntax highlighting

### Design Decisions

1. **Manual Tokenizer** (not tree-sitter): Reuse existing `tokenize_json_line` from response.rs
2. **Simplified Architecture**: No complex DisplayMap layers - just buffer + highlighting
3. **Small Files**: Each file <300 lines, clear single responsibility
4. **Follow Zed Patterns**: Element trait, paint layering, viewport culling

### File Structure (~800 lines total)

```
crates/api-dash/src/ui/components/code_editor/
├── mod.rs           (~150 lines) - Public API, CodeEditor struct
├── buffer.rs        (~120 lines) - Text buffer, line operations
├── highlight.rs     (~180 lines) - Tokenizers (JSON, XML, Plain)
├── selection.rs     (~100 lines) - Cursor, selection handling
├── render.rs        (~150 lines) - GPUI rendering, viewport culling
└── input.rs         (~100 lines) - Mouse/keyboard handlers
```

### Progress

- [x] Phase 1: Text Buffer (`buffer.rs`) - 120 lines
- [x] Phase 2: Syntax Highlighting (`highlight.rs`) - 235 lines (JSON, XML, Plain)
- [x] Phase 3: Selection (`selection.rs`) - 130 lines
- [x] Phase 4-6: Core Component (`mod.rs`) - 570 lines (combined struct, render, input)
- [ ] Phase 7: Integration - Wire into request/response panels

**Note:** Phases 4-6 were combined into a single mod.rs file since GPUI's patterns
favor keeping related functionality together. Total: ~1055 lines across 4 files.

### Component Features
- Syntax highlighting for JSON, XML, HTML, Plain text
- Line numbers with gutter
- Selection with click, drag, double-click (word), triple-click (line)
- Cursor navigation (arrows, home, end)
- Text editing (insert, delete, backspace)
- Copy/Cut/Paste support
- Read-only mode option
- Configurable font size and line height

### Key Files to Modify

| File | Change |
|------|--------|
| `ui/components/mod.rs` | Export `code_editor` module |
| `ui/panels/request/mod.rs` | Use `CodeEditor` for body |
| `ui/panels/request/render.rs` | Remove `render_body_text`, use component |
| `ui/panels/response.rs` | Move `tokenize_json_line` to highlight.rs, use `CodeEditor` |


### Other plans location
~/.claude/plans/golden-swinging-porcupine.md
~/.claude/plans/binary-greeting-hennessy.md
