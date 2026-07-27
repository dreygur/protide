# Protide - Development Context

@.claude/preferences.md

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
- JetBrains Mono font (bundled in `crates/protide/assets/fonts/`, registered via `add_fonts`)
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
│   │           │   ├── text_view.rs
│   │           │   ├── ui_helpers.rs
│   │           │   └── word_select.rs   # Double-click word/separator spans
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
└── website/                            # Marketing site + docs (Next.js 16 + Nextra 4)
    ├── next.config.mjs                 # static export, basePath /protide, contentDirBasePath /docs
    └── src/
        ├── app/                        # / landing, /docs catch-all, sitemap
        ├── components/landing/         # landing page React components + UI mockups
        ├── content/                    # docs MDX + _meta.ts sidebar config
        └── styles/                     # tokens.css (light-dark() mirror of theme.rs)
```

### Key Technical Decisions
1. **HTTP requests**: `reqwest::blocking::Client` in background thread - GPUI doesn't play well with tokio async in UI code
2. **File format**: Extended .http file format with annotations (`# @name`, `# @protocol`, etc.)
3. **No database**: File-system based storage (collections = folders)
4. **UI framework**: GPUI from Zed - GPU-accelerated, immediate mode style
5. **Collaboration**: Local-first CRDT, no central server required
6. **MCP**: JSON-RPC 2.0 over stdio - lets AI tools (Claude, etc.) drive requests
7. **Website**: Next.js + Nextra (docs theme), statically exported to GitHub Pages. `zod` is pinned
   to `~4.1` in `website/package.json` overrides - zod 4.4 breaks `nextra-theme-docs`' prop schemas.

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
make run              # Release build recommended
```

### Verification

`make verify` is the gate. Run it before pushing; it is what the git hooks run.

```bash
make verify     # fmt-check -> clippy (-D warnings) -> test -> audit
make hooks      # one-time: enable .githooks (pre-commit fmt+clippy, pre-push verify)
```

- **727 tests** (90 http-parser + 308 protide-core + 229 protide-ui + 98 lsp + 2 mcp),
  none `#[ignore]`d — the five recorded defects are fixed, see **Fixed Defects** below.
  `protide-core` drops tests without `--features full-sync`; the PAKE tests are
  `#![cfg(feature = "pake-auth")]`.
- Use `execution/test_server.rs` (`TestServer`) for anything needing a real HTTP
  socket. Never hit an external endpoint from a test.
- Targets run **crate-by-crate on purpose**. A single `--workspace` invocation unifies
  features across members and can pull in a native-TLS backend needing system packages
  beyond `make deps`. Do not collapse them.
- Tests must not use fixed temp paths or unbounded waits. Use the shared `TempDir` guard
  (`protide-core/src/test_support.rs`) and always set socket/client timeouts.
- `[workspace.lints]` in the root `Cargo.toml` carries `allow` entries with rationale.
  Fix the code and delete the entry rather than adding new ones.

### Fixed Defects

The five defects formerly parked as `#[ignore]`d tests are fixed. Each fix required a
behaviour decision, recorded below and in a `REGRESSION:` / `FIXED:` comment on the test
(the convention set by the convergence test in `protide-core/src/sync/crdt.rs`).

**These decisions are load-bearing — do not reverse them without replacing the reasoning.**

| # | Test | Decision taken |
|---|---|---|
| 1 | `a_variable_prefixed_url_on_its_own_line_is_recognised` | A `{{`-prefixed line is a URL **only in URL position** (a bare `Token::Method` was just emitted). The guard is positional, never a blanket `{{` allowance — a body is never in URL position, so a request whose whole body is `{{payload}}` still lexes as a body. That property is now pinned by `a_body_that_is_only_a_variable_stays_a_body`; do not weaken the guard to a plain `starts_with("{{")`. URL position lasts exactly one consumed line, so `GET` / comment / URL stays a parse error. |
| 2 | `set_with_a_blank_name_defines_nothing` | `# @set  = $.token` falls through to the generic annotation path (which the parser ignores), exactly as a `@set` with no `=` already did. |
| 3 | `a_missing_url_is_blamed_on_the_request_line` | `MissingUrl` blames `start_line` (the request line) rather than the token in hand. This deliberately shifts the non-EOF case from the header's line to the request line; the EOF case no longer points past the end of the document, so protide-lsp can no longer emit a range outside the file. |
| 4 | `a_double_click_on_a_separator_selects_only_one_word` | Double-clicking a separator selects **the run of separators** (the VS Code / Zed convention), not a neighbouring word. Chosen, not derived — "select the following word" is equally self-consistent. `find_word_start` / `find_word_end` both delegate to one direction-agnostic `word_span`, so the two halves cannot disagree again. |
| 5 | `a_double_click_selects_a_whole_grapheme_cluster` | Classification is per grapheme cluster (`unicode-segmentation`), but the returned **indices stay codepoint-based**. Every caller mixes them with char-counted values (`index_for_x`, `select_all`, `normalized_selection`) and the renderer multiplies them by a per-char width, so returning grapheme indices would mis-position every cursor over multibyte text. |

Files: 1–3 in `crates/http-parser/src/adversarial_tests.rs`, 4–5 in
`crates/protide-ui/src/components/word_select.rs` (split out of `text_view.rs`, which was
already over the 333-line budget).

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
