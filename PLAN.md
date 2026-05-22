Fix All Identified Issues

 Context

 Post-audit identified four categories of problems: correctness bugs in the sync engine, production panics from unwrap(), custom UI components duplicating gpui-component, and 333-line rule
 violations. This plan addresses all of them in dependency order — correctness first, then component migration (which also fixes the largest file-size violations), then remaining splits and
 cleanup.

 ---
 Phase 1 — Correctness Bugs (Surgical, Non-Breaking)

 1a. Fix NodeId persistence

 Problem: SyncEngine::new() at sync/mod.rs:82 always calls NodeId::new(). FileSync::open() at file_sync.rs:52-56 writes the node_id to .protide/node_id but never reads it back. Every app
 restart is a new node identity — LWW resolution breaks across sessions.

 Fix:
 - Add node_id_path: Option<PathBuf> to SyncConfig in sync/types.rs
 - Add helper fn load_or_create_node_id(path: &Path) -> NodeId in sync/types.rs: read from file if exists, else NodeId::new() + write
 - In SyncEngine::new(), call this helper when config.node_id_path is set
 - In main.rs:84-90, set node_id_path: Some(dirs::config_dir().unwrap_or_default().join("protide/node_id"))

 1b. Fix wall-clock-as-Lamport comment

 Problem: sync/types.rs comment says /// Lamport timestamp (milliseconds since epoch) — contradictory. The max(timestamp_now(), existing.timestamp + 1) pattern in update_local() and
 delete_local() already ensures local monotonicity; cross-machine clock skew is the real risk.

 Fix: Change the field comment to /// wall-clock timestamp (ms since epoch); cross-machine skew is a known LWW limitation. Update the module-level comment in sync/mod.rs:13 to remove
 "Lamport" — this is LWW-with-wall-clock, not causal CRDT. No logic change needed; this is documentation correctness.

 1c. Delete CrdtEntry::tombstone()

 Problem: sync/types.rs:69-79 — dead function (zero production call sites), hardcodes DataType::Request regardless of deleted entry type. The real deletion path (CrdtStore::delete_local())
 already reads the correct data_type from the existing entry.

 Fix: Delete CrdtEntry::tombstone() entirely.

 1d. Fix blocking HTTP inside async executor

 Problem: graphql.rs:19: cx.background_executor().spawn(async move { run_graphql_introspection(&url) }) — GPUI's background executor is smol-based. run_graphql_introspection calls
 reqwest::blocking::Client. Blocks a smol thread pool thread.

 Fix: Match the pattern used by execution_http.rs:186 — use std::thread::spawn with a channel:
 let (tx, rx) = std::sync::mpsc::channel();
 std::thread::spawn(move || { let _ = tx.send(run_graphql_introspection(&url)); });
 cx.spawn(async move |this, cx| {
     let result = cx.background_executor().spawn(async move { rx.recv().unwrap_or(GraphqlSchemaState::Error("thread error".into())) }).await;
     // ... update UI
 }).detach();

 1e. Fix production unwraps (priority order)

 mock_server/server.rs (highest risk — can crash mock server):
 - Line 51: state.routes.read().unwrap() → .read().unwrap_or_else(|e| e.into_inner())
 - Lines 87, 114, 165, 171: Response::builder()...body(...).unwrap() → use expect("infallible builder") at minimum; prefer unwrap_or_else with a static fallback response

 mock_server/mod.rs:
 - Line 128: listener.local_addr().unwrap() → return error via Result
 - Line 206: result.unwrap() → log error and return early

 openapi/schema.rs:
 - Lines 18, 33: example.as_str().unwrap() → .as_str().unwrap_or_default()

 highlight.rs (can crash UI on malformed input):
 - Lines 250, 252, 257, 259: chars.next().unwrap() — add length guard before each call or use chars.next()? pattern with early return

 http-parser/ast.rs:176: self.body.as_ref().unwrap() — add if self.body.is_none() { return "" } guard upstream or use unwrap_or_default()

 lsp/formatting.rs:73: body_lines.last().unwrap() → body_lines.last().copied().unwrap_or(0) as u32

 ---
 Phase 2 — Replace Custom Components with gpui-component

 gpui-component (already a workspace dep) provides:
 - gpui_component::Input — full text input with prefix/suffix, masking, sizes
 - gpui_component::Input with InputMode::CodeEditor — tree-sitter highlighting, line numbers, LSP integration
 - gpui_component::Dialog / AlertDialog — modal dialogs

 2a. Migrate TextInput entity usages → gpui-component Input

 Only 4 Entity<TextInput> usages exist outside text_input.rs:

 ┌────────────────────────┬──────────────────────────────┬─────────────────────┐
 │          File          │            Field             │        Notes        │
 ├────────────────────────┼──────────────────────────────┼─────────────────────┤
 │ response/mod.rs:92,124 │ jsonpath_input, search_input │ single-line inputs  │
 ├────────────────────────┼──────────────────────────────┼─────────────────────┤
 │ request/init.rs:194    │ timeout_input                │ single-line numeric │
 ├────────────────────────┼──────────────────────────────┼─────────────────────┤
 │ mock_server/mod.rs:17  │ status_input                 │ single-line numeric │
 └────────────────────────┴──────────────────────────────┴─────────────────────┘

 Replace Entity<TextInput> with Entity<gpui_component::Input>. Update construction and read calls (.text() / .set_text()).

 The render_text_view_with_max and render_text_view_with_max_scrolled functions in text_input.rs are standalone render functions (no TextInput state dependency). Extract them to a new
 components/text_view.rs (~100 lines). They are used in:
 - explorer/render_inputs.rs (3 call sites)
 - request/render_url_bar.rs (1 call site)
 - request/render_kv.rs (1 call site)

 2b. Migrate CodeEditor → gpui-component Input/CodeEditor mode

 3 Entity<CodeEditor> usages:

 ┌────────────────────┬───────────────────┬───────────────────┐
 │        File        │       Field       │       Mode        │
 ├────────────────────┼───────────────────┼───────────────────┤
 │ response/mod.rs:74 │ body_viewer       │ read-only display │
 ├────────────────────┼───────────────────┼───────────────────┤
 │ response/mod.rs:96 │ extraction_editor │ editable          │
 ├────────────────────┼───────────────────┼───────────────────┤
 │ request/init.rs:10 │ body_editor       │ editable          │
 └────────────────────┴───────────────────┴───────────────────┘

 Important: Verify gpui-component Input supports read-only mode before migrating body_viewer. If it does, use it; if not, keep body_viewer as a custom read-only viewer (a simpler subset of
 the current code).

 Language mapping: current Language enum (Json, JavaScript, GraphQL, Http) → gpui-component tree-sitter grammar names. Confirm grammars are available in the checked-out version at
 ~/.cargo/git/checkouts/gpui-component-*/.

 2c. Migrate modal.rs → gpui-component Dialog

 modal.rs is used in mock_server/mod.rs. Replace ModalState / Modal with gpui_component::Dialog. Update call sites.

 2d. Delete migrated custom components

 After migration:
 - Delete components/text_input.rs (currently 1071 lines + #![allow(dead_code)])
 - Delete components/code_editor/ directory (879 + 582 + buffer.rs + selection.rs lines + #![allow(dead_code)] on all)
 - Delete or reduce components/modal.rs
 - Update components/mod.rs exports

 This eliminates ~3000 lines and 5+ #[allow(dead_code)] suppressions in one pass.

 ---
 Phase 3 — Split Remaining Oversized Files

 After Phase 2, remaining 333-line violations:

 ┌───────────────────────┬───────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┐
 │         File          │ Lines │                                                    Split into                                                    │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ panels/console.rs     │ 770   │ console_entry.rs (types + ConsoleEntry impl), console_render.rs (Render impl), console.rs (struct + thin wiring) │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ import/openapi/mod.rs │ 735   │ openapi_security.rs, openapi_paths.rs, openapi_operations.rs — tests stay in mod.rs                              │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ scripting/runtime.rs  │ 614   │ scripting/bindings.rs (all the setup_*_js fns)                                                                   │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ protocols/grpc.rs     │ 602   │ grpc_encoding.rs (grpc_encode_message, grpc_decode_message)                                                      │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ panels/presence.rs    │ 588   │ presence_render.rs (Render impl blocks)                                                                          │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ sync/mod.rs           │ 538   │ sync/engine.rs (SyncEngine impl methods)                                                                         │
 ├───────────────────────┼───────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┤
 │ execution/sio.rs      │ 504   │ sio_codec.rs (SIO encoding/decoding)                                                                             │
 └───────────────────────┴───────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────────────┘

 Pattern: keep struct definitions in the original file, extract impl blocks to _render.rs / _ops.rs / named submodule files.

 ---
 Phase 4 — Dead Code Cleanup

 After Phases 2–3:
 1. Remove #![allow(dead_code)] from theme.rs (used in 20+ files; suppression is hiding real unused variants — fix them individually)
 2. Remove #![allow(dead_code)] from models/request.rs and models/environment.rs — use #[serde(default)] for deserialize-only fields instead
 3. Remove item-level #[allow(dead_code)] in request_types.rs — delete the "completeness" variants that are truly unused
 4. Remove #![allow(dead_code)] from history.rs, runner/mod.rs, mock_server/mod.rs once dead fields are deleted

 ---
 Critical Files

 crates/protide-core/src/sync/types.rs          — 1a, 1b, 1c
 crates/protide-core/src/sync/mod.rs            — 1a, Phase 3
 crates/protide/src/main.rs                     — 1a (config update)
 crates/protide-ui/src/panels/request/graphql.rs — 1d
 crates/protide-core/src/mock_server/server.rs  — 1e
 crates/protide-core/src/mock_server/mod.rs     — 1e
 crates/protide-core/src/import/openapi/schema.rs — 1e, Phase 3
 crates/protide-ui/src/components/code_editor/highlight.rs — 1e
 crates/http-parser/src/ast.rs                  — 1e
 crates/protide-lsp/src/formatting.rs           — 1e
 crates/protide-ui/src/panels/response/mod.rs   — Phase 2
 crates/protide-ui/src/panels/request/init.rs   — Phase 2
 crates/protide-ui/src/panels/mock_server/mod.rs — Phase 2
 crates/protide-ui/src/components/text_input.rs — DELETE after Phase 2a
 crates/protide-ui/src/components/code_editor/  — DELETE after Phase 2b
 crates/protide-ui/src/components/modal.rs      — DELETE after Phase 2c
 crates/protide-ui/src/panels/console.rs        — Phase 3
 crates/protide-core/src/scripting/runtime.rs   — Phase 3

 ---
 Verification

 cargo test                    # must pass — ~190 tests
 cargo build --release         # must compile clean, zero warnings
 cargo run --release           # smoke test:

 Manual checks after cargo run --release:
 1. URL bar types correctly (TextInput → gpui-component Input)
 2. Request body editor has syntax highlighting (CodeEditor → gpui-component)
 3. Response body viewer renders JSON (read-only CodeEditor or new viewer)
 4. GraphQL introspection: enter URL, click "Fetch Schema" — UI stays responsive
 5. Restart app twice with sync_folder set → verify same node_id in .protide/node_id
 6. Mock server: start server, configure a bad route, verify it logs error instead of panicking
