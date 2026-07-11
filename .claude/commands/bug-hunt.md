Orchestrate a parallel, multi-agent bug hunt across the Protide codebase. Optional argument `$ARGUMENTS` scopes the hunt to one feature area (see list below); if empty, run the full sweep.

## Ground truth before you start

- Protide is a **native desktop app** (Rust + GPUI), not a web app — there is no browser to drive. Do not reach for claude-in-chrome or any browser tool.
- This machine currently has **no desktop-automation or screenshot tooling installed** (no xdotool, wmctrl, scrot, gnome-screenshot, ffmpeg) and runs a **Wayland** session, so even if some were installed, X11 tools like xdotool/wmctrl would not reliably work. That means pixel-level visual glitches (misalignment, the dark-rendering-artifact issue from stray spacer divs, overflow_scroll clipping) **cannot** be caught by this command — they need a human looking at the running app. Say so plainly in your final report; do not pretend to have visually inspected anything.
- What *does* work here, and is the backbone of this hunt: (1) close reading of the source for logic bugs, and (2) real interaction tests using GPUI's own test harness (`gpui::TestAppContext`, `#[gpui::test]`), which already exists in this repo at `crates/protide-ui/src/panels/response/tests_gpui.rs` and `crates/protide-ui/src/main_window/mod.rs`. Follow those files' patterns exactly when writing new interaction tests — don't invent a new harness.
- There is no existing bug tracker in this repo (no TODO.md/ISSUES.md, negligible TODO/FIXME comments) — you are the first pass. Read `CLAUDE.md` for architecture, the GPUI gotchas list, and file-layout before diving in.

## Step 0 — cheap wins first

Run these before spawning anything, and fold real failures into your final report:
```
cargo clippy --workspace --all-targets --all-features 2>&1 | tee /tmp/clippy.out
cargo test --workspace 2>&1 | tee /tmp/test.out
```
Clippy alone usually surfaces a chunk of the panic-prone patterns (needless unwraps, etc.) below for free.

## Step 1 — split into feature-area subagents

Each subagent gets ONE of these areas (skip to the one named in `$ARGUMENTS` if given). Spawn them with the general-purpose Agent tool, in parallel batches of ~4-5 so you don't drown in concurrent output.

| Area | Primary paths |
|---|---|
| HTTP request panel (headers/params/body/auth) | `crates/protide-ui/src/ui/panels/request/` |
| Response panel + JSON tree | `crates/protide-ui/src/ui/panels/response/` |
| GraphQL / WebSocket / Socket.IO | `crates/protide-core/src/execution/`, relevant `panels/request/` submodules |
| gRPC / tRPC | `crates/protide-core/src/protocols/`, relevant `panels/request/` submodules |
| Collections, Explorer, Environments | `crates/protide-ui/src/ui/panels/explorer/` |
| Scripting engine (pre/post-request JS, `expect()`) | `crates/protide-core/src/scripting/` |
| Import/Export (curl, Postman, Bruno, OpenAPI, Markdown) | `crates/protide-core/src/import/`, `src/export/` |
| Request chaining (JSONPath, `@set`) | `crates/protide-core/src/chaining/` |
| Code generation | `crates/protide-core/src/codegen/` |
| Mock server (routes, record/proxy mode) | `crates/protide-core/src/mock_server/`, `panels/mock_server/` |
| Collaboration/sync (CRDT, P2P, PAKE, BYOB) | `crates/protide-core/src/sync/`, `panels/presence.rs` |
| LSP server | `crates/protide-lsp/` |
| MCP server | `crates/protide-mcp/` |
| Core UI infra (code_editor, text_input, modal, action_row, main_window, theme) | `crates/protide-ui/src/ui/components/`, `ui/main_window/`, `theme.rs` |

Each subagent brief must include, verbatim:

> You are hunting for real, demonstrable bugs in `<paths>` — not style nits. For each suspicious spot, decide if it's static or dynamic:
>
> **Static red flags to grep/read for:** `.unwrap()`/`.expect()`/`panic!`/direct indexing on anything derived from network input, file contents, or user text; unhandled `Result`/`Option` that silently drops errors; state that can desync between the UI model and the underlying data (e.g. query-param editor vs URL string); races across the background-thread ↔ GPUI-entity boundary (HTTP runs via `reqwest::blocking` in a background thread per `CLAUDE.md` — check every channel/callback for use-after-close, double-sends, or dropped results); resource leaks (WS/gRPC/SocketIO streams or file handles not torn down on disconnect/panel-close); off-by-one or boundary bugs in list/tree rendering; env-var `{{variable}}` substitution edge cases (empty, recursive, unknown var, malformed braces); parser edge cases in import/codegen for malformed or adversarial input.
>
> **When you find a plausible one:** try to *prove* it with a real interaction test, not just a hunch. Write a throwaway `#[gpui::test]` following the exact pattern in `crates/protide-ui/src/panels/response/tests_gpui.rs` (or `main_window/mod.rs` for entity-lifecycle bugs) that drives the component into the failing state and asserts on the actual result (or watch it panic). Run it. If it confirms the bug, keep the test as a regression test only if it's clean and minimal; otherwise delete it. If a bug can't be exercised via `gpui::test` (e.g. real-network gRPC/WS/SocketIO behavior), you may use the local mock server (`crates/protide-core/src/mock_server`) or the existing `e2e/*.http` fixtures as safe local targets — never hit real external services.
>
> Report each confirmed or highly-plausible finding as: file:line, one-sentence defect, concrete failure scenario (exact input/sequence that triggers it), whether you reproduced it with a test (and if so, did you keep the test), and a severity guess (crash/data-loss > incorrect-behavior > cosmetic-but-real > nit). Do not report anything you didn't verify by reading the actual current code — no guessing from the file tree alone.

## Step 2 — consolidate

Collect every subagent's findings yourself. Dedupe overlapping reports, drop anything that turned out to be a false positive on closer look, sort by severity, and produce one final list. Use `ReportFindings` if available; otherwise a clean markdown summary. Explicitly call out:
- the clippy/test failures from Step 0 (if any),
- how many findings were confirmed via a `gpui::test` repro vs static-only,
- the visual/pixel-glitch gap (cannot be automated in this environment — needs a human running `cargo run --release` and looking).

## Guardrails

- Don't fix anything while hunting — report only, unless a fix is a one-line, unambiguous typo-level correction and you note it explicitly as such.
- Don't touch real external network endpoints; use the mock server or `e2e/*.http` fixtures.
- Don't install new system packages (e.g. `ydotool`/`grim` for future visual testing) without asking the user first — that's a separate decision, not part of this hunt.
