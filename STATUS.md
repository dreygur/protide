# Protide Pre-Flight Audit - May 2025

**Verdict: NO-GO.** Three hard blockers exist before external Alpha tag. The core architectural migration is genuine and verified. The holes are surgical and fixable in one session.

---

## Summary Table

| Dimension | Status | Blocking? |
|---|---|---|
| Dependency boundary (`protide-ui` zero-hit for reqwest) | GREEN | - |
| HTTP execution flow | RED | Yes - no timeout |
| GraphQL variables error handling | YELLOW | No |
| WebSocket state machine | GREEN | - |
| WS ring buffer memory bound | GREEN | - |
| gRPC unary | GREEN | - |
| gRPC streaming (`use_h2c` bug) | RED | Yes - TLS endpoints fail |
| gRPC streaming timeout | YELLOW | No |
| JS scripting - pre-script errors | GREEN | - |
| JS scripting - post-script/test errors | YELLOW | No |
| JS sandbox execution time limit | RED | No (post-Alpha) |
| MCP schema accuracy | RED | Yes - documents nonexistent API |
| Explorer virtualization | GREEN | Done |
| TLS / libssl-dev | GREEN | rustls, no system dep |
| GPUI Linux env documented | YELLOW | No |

---

## 1. Dependency Audit - protide-ui "zero-hit" Claim

**VERIFIED.**

`crates/protide-ui/Cargo.toml` has no `reqwest`, `tokio-tungstenite`, or `futures-util`. Zero occurrences of `reqwest` in `crates/protide-ui/src/`. The execution call chain is cleanly:

```
RequestPanel::send_request()
  → std::thread::spawn(|| protide_core::execution::execute(req))
      → execution/http.rs → reqwest::blocking::Client
```

**One residual structural issue (non-crash):**

`mod.rs:455` calls `TungsteniteExecutor::connect(...)` by name. The `WebSocketExecutor` trait exists in `ws.rs:113` but is never used polymorphically - the UI hard-codes the concrete type. The trait is inert. WS execution is untestable in isolation without refactoring.

---

## 2. State Machine Audit

### Finding A - No HTTP request timeout [BLOCKER]

**File:** `crates/protide-core/src/execution/http.rs:51`

```rust
let client = reqwest::blocking::Client::new();  // no .timeout() configured
```

Also affects: `protocols/trpc.rs:54` (same pattern, same risk).

A request to a black-holed endpoint blocks the background thread forever. The `loading` spinner never clears. Each additional Send click spawns another leaked OS thread. There is no cancellation mechanism - `task.detach()` means the spawner has no handle to abort it.

**Fix:** `reqwest::blocking::Client::builder().timeout(Duration::from_secs(30)).build()`

---

### Finding B - gRPC streaming `use_h2c` dead variable [BLOCKER]

**File:** `crates/protide-core/src/protocols/grpc.rs:276`

```rust
let use_h2c = http_url.starts_with("http://");  // computed, never read

let client = reqwest::Client::builder()
    .http2_prior_knowledge()   // unconditional - wrong for TLS endpoints
    .build()?;
```

This affects `execute_server_streaming`, `execute_client_streaming`, and `execute_bidi_streaming`. `.http2_prior_knowledge()` forces H2 on a plaintext connection; for `grpcs://` (TLS) the client must negotiate H2 via ALPN, not assert prior knowledge. All TLS gRPC streaming endpoints will fail with a connection error.

The unary blocking path at lines 116–122 is correct - the conditional is there. The async paths copied the variable and forgot the branch.

**Fix:** Apply the same conditional already present in `execute_unary_blocking`:

```rust
let mut builder = reqwest::Client::builder();
if use_h2c {
    builder = builder.http2_prior_knowledge();
}
let client = builder.build()?;
```

---

### Finding C - MCP schema documents a nonexistent API [BLOCKER]

**File:** `crates/protide-mcp/src/main.rs:141,149`

```
"pre_script": "JavaScript pre-request script (pm.request, pm.environment APIs available)"
"tests": "JavaScript test assertions (pm.test, pm.expect APIs)"
```

The scripting runtime exposes `request.*`, `response.*`, `env.*`, and `expect()`. It does **not** expose `pm`. Any agent that reads this schema and generates:

```js
pm.test("status is 200", () => { pm.expect(response.status).to.eql(200); })
```

will receive `ReferenceError: pm is not defined`. The agentic MCP integration is a stated differentiator; shipping a schema that lies about its own API is a day-one credibility failure.

**Fix:** Replace `pm.request`, `pm.environment`, `pm.test`, `pm.expect` in both description strings with the actual surface: `request.setHeader()`, `request.setUrl()`, `env.set()`, `env.get()`, `expect(value).toBe()`.

---

### Finding D - Silent post-script and test failures

**File:** `crates/protide-core/src/execution/mod.rs:148–170`

```rust
if let Ok(engine) = ScriptEngine::new() {       // failure silently dropped
    ...
    if let Ok(outcome) = engine.run_post_script(...) {  // failure silently dropped
```

Pre-script failures correctly propagate via `?` and surface as errors. Post-script and test failures are swallowed. The user gets no console output and no indication anything went wrong - asymmetric contract that will confuse anyone debugging scripts.

---

### Finding E - GraphQL variables parse failure is a silent data corruption

**File:** `crates/protide-core/src/execution/http.rs:29`

```rust
serde_json::from_str(variables).unwrap_or(serde_json::json!({}))
```

Invalid JSON in the variables editor fires the request with `variables: {}` and no user feedback. The query returns unexpected results with no error surface.

---

### Finding F - No JS sandbox execution time limit

**File:** `crates/protide-core/src/scripting/runtime.rs`

`rquickjs` is initialized with no interrupt handler or CPU deadline. A pre-script containing `while(true){}` spins the `std::thread::spawn` background thread indefinitely - HTTP is never sent, the spinner never clears, the thread cannot be killed from the UI. Each Send click accumulates a leaked OS thread.

`rquickjs 0.6.x` exposes `Runtime::set_interrupt_handler` which can implement a cycle budget. Not a hard blocker for internal Alpha but must be addressed before any public user exposure.

---

### Finding G - gRPC streaming has no timeout

**File:** `crates/protide-core/src/protocols/grpc.rs` (async streaming functions)

`execute_server_streaming`, `execute_client_streaming`, `execute_bidi_streaming` all build `reqwest::Client` with no `.timeout()`. A server that sends one chunk and stalls hangs the GPUI async task forever; `loading` never clears.

---

### Finding H - Bare `expect()` in WS background thread

**File:** `crates/protide-core/src/execution/ws.rs:131`

```rust
let rt = tokio::runtime::Runtime::new().expect("ws tokio runtime");
```

Panics on the background thread if OS thread/fd limits are hit. The UI event loop survives (channel drops → `RecvTimeoutError::Disconnected` → cleanup), but the UX is a silent "connection failed" with no message. Low probability, but trivially fixable with `.map_err(|e| { let _ = event_tx.send(WsEvent::Error(...)); })`.

---

## 3. Memory & Performance - WS Ring Buffer

**Solid.** `VecDeque<WsMessage>` with `cap=1000`, `push()` evicts `pop_front()` at capacity, pre-allocated via `with_capacity`. Two unit tests cover eviction and clone independence. Memory ceiling is count-bounded; worst case ~1MB for 1KB messages.

**One residual note:** `WsMessage.content: String` has no per-message byte limit - only a message count limit. A single 50MB frame occupies one slot in the ring. Acceptable for Alpha localhost usage.

**Explorer tree virtualization: complete.** `explorer.rs:1372–1404` implements correct viewport virtualization: `start_idx`, `visible_count`, spacer divs for top/bottom offset.

---

## 4. Alpha Criteria Checklist

| Criterion | Status | Notes |
|---|---|---|
| HTTP/GraphQL execution feature-complete | ✅ | All methods, auth, body types, env substitution |
| GraphQL malformed variables | ⚠️ | Silent substitution with `{}` |
| WebSocket state machine | ✅ | States correct, ring buffer bounded |
| gRPC unary | ✅ | 30s timeout, h2c conditional correct |
| gRPC streaming | ❌ | `use_h2c` bug, no timeout |
| tRPC | ⚠️ | Functional; no timeout; no tRPC v11 batch |
| `ExecutionResult` serializable for MCP | ✅ | `serde::Serialize` derived |
| MCP schema accuracy | ❌ | `pm.*` documented, does not exist |
| TLS - no `libssl-dev` required | ✅ | `rustls-tls` / `rustls-tls-native-roots` |
| No `protoc` required | ✅ | `protox` handles compilation internally |
| Request cancellation | ❌ | Not implemented |
| GPUI Linux system deps documented | ❌ | Wayland/X11 libs, GPU drivers not documented |

---

## Blockers - Must Fix Before Alpha Tag

| # | File | Issue |
|---|---|---|
| 1 | `crates/protide-mcp/src/main.rs:141,149` | MCP schema documents `pm.*` API that does not exist - replace with actual `request.*` / `env.*` / `expect()` surface |
| 2 | `crates/protide-core/src/execution/http.rs:51` + `protocols/trpc.rs:54` | No HTTP timeout - add `Client::builder().timeout(Duration::from_secs(30))` |
| 3 | `crates/protide-core/src/protocols/grpc.rs:278–281` (and equiv. in client/bidi streaming) | `use_h2c` dead variable - apply conditional `.http2_prior_knowledge()` matching the unary path |

---

## Post-Alpha Critical Tech Debt

**1. JS sandbox execution time limit**
`rquickjs::Runtime::set_interrupt_handler` can implement a cycle budget. One `while(true){}` in a user pre-script currently leaks an OS thread permanently. Implement a ~5s wall-clock deadline via the interrupt handler.

**2. Symmetric script error handling**
Post-script and test failures must surface with the same fidelity as pre-script failures. Replace `if let Ok(outcome) = engine.run_post_script(...)` with explicit error propagation into `ExecutionResult::console_output`.

**3. `WebSocketExecutor` trait - make it load-bearing**
Inject the executor into `RequestPanel` rather than constructing `TungsteniteExecutor` by name. The trait exists but is currently inert. Without this, the WS execution path has zero integration test coverage at the UI level.

---

## Market Position (unchanged - GREEN)

| Dimension | Protide | Postman / Insomnia |
|---|---|---|
| Runtime | Native Rust, ~10MB binary | Electron, ~300MB+, slow cold start |
| Memory footprint | <50MB typical | 400–800MB |
| GPU rendering | GPUI (same as Zed) | Chromium-based |
| gRPC | Native, no protoc needed | Requires protoc or reflection API |
| tRPC | Present (unique) | Not supported |
| JS scripting | rquickjs, sandboxed | V8 in Electron |
| Mock server | Present | Postman paid tier only |
| Local-first | File-based, git-friendly | Cloud sync required |

**Gaps vs. established tools:** no OAuth 2.0 flows, no mTLS UI, no batch collection runner, no response diffing.
