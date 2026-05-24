# Code Review Walkthrough — `serial-mcp-server`

## Context

Reading guide for the v0.2.0 fork, now that the sprint sequence
(A–K plus integration tests) is done. This document is the reading
order plus per-file checkpoints — the things actually worth your
attention vs. the things that are just plumbing.

State of the repo, head of `main` = `9b4f4a9`:

```
src/
  main.rs            28 LOC   stdio binary entry
  bin/http.rs        64 LOC   streamable-HTTP binary entry
  lib.rs              7 LOC   module declarations + re-exports
  error.rs           27 LOC   single SerialError enum
  codec.rs          184 LOC   Encoding enum + decode/encode + 9 tests
  serial.rs         600 LOC   PortInfo, SerialIo trait, ConnectionManager
                              + LoopbackIo test backend + 12 tests
  handler.rs       1342 LOC   11 tools, 3 resources, 2 prompts,
                              all response structs, parse helpers,
                              streaming + task framework wiring, 15 tests
tests/
  common/mod.rs     213 LOC   TestServer harness + NotificationCollector
                              + PtyPair helper
  http_integration  331 LOC   Layer 2 — 14 in-memory HTTP tests
  serial_pty        217 LOC   Layer 3 — 6 real-PTY tests (Linux only)
  hardware_loopback 186 LOC   Layer 4 — 2 ignored hardware tests
```

Total ~3.2 KLOC production+tests. Active: 56 tests pass, clippy
`-D warnings` clean, `cargo fmt --check` clean.

## Reading order (start → finish, ~60–90 min)

The files are listed in the order that makes the design easiest to
absorb. Each item is **what to read**, then **questions to ask
yourself while reading**.

### 1. `Cargo.toml` — start here

- What each dep is for (rmcp = SDK, axum = HTTP runtime, tokio-serial
  = the actual serial port driver, schemars = JSON schema for tool
  args).
- Two `[[bin]]` entries — confirm you understand why we have a stdio
  binary + an HTTP binary sharing one library.
- `[features]` (no custom features yet) — relevant if you later want
  a `test-support` feature flag.
- Dev-deps split between unix-only (nix) and cross-platform (rmcp
  client, tokio process/fs, anyhow).

### 2. `src/lib.rs` — module map

Six lines. Confirms the public surface: `Result`, `SerialError`,
`SerialHandler`. Everything else (`codec`, `serial::*`,
`handler::*`) is reachable via path.

### 3. `src/main.rs` — stdio entry

28 lines. Single tokio::main, logging setup, `SerialHandler::new()`
on stdio.

**Watch for:** no CLI flags. The only knob is `RUST_LOG`. Question:
do you want a `--port-allowlist` or similar at some point, or is that
a security policy that belongs upstream of the binary?

### 4. `src/bin/http.rs` — HTTP entry

64 lines. The streamable-HTTP variant. Mount path `/mcp`, env
`SERIAL_MCP_HTTP_BIND` overrides `127.0.0.1:8000`.

**Watch for:**
- Ctrl-C → cancellation token → graceful drain. Verify the shutdown
  branches make sense.
- No TLS, no auth, no Origin check (rmcp 1.6+ adds Origin validation,
  default config). Is `127.0.0.1` default enough, or do you want to
  flip the default to require an explicit bind override?
- The `move || Ok(SerialHandler::new())` factory — every new HTTP
  session gets its own handler instance, which means each session
  has its own `ConnectionManager`. That's likely *not* what you want
  in production (two clients can both open the same physical port).

### 5. `src/error.rs` — error model

27 lines. Single `SerialError` with 7 variants. All `#[from]`
conversions are explicit, no swallowed errors.

**Watch for:** `IoError(std::io::Error)` is the catch-all. Inspect
whether any operational error gets silently mapped to it where a
more specific variant would carry more information for the LLM.

### 6. `src/codec.rs` — UTF-8 / hex / base64

184 lines. `Encoding` enum + `decode/encode` + 9 unit tests.

**Watch for:**
- Hex decoder strips spaces (`"48 65 6c"` works). Confirm you want
  that lenience.
- Base64 falls back to URL-safe-no-padding after standard base64
  fails. Two attempts on every decode is fine for the volumes
  involved but worth noting.
- `Encoding::from_str` returns `CodecError`; in `handler.rs` callers
  wrap that string into the tool error. Spot-check the
  `parse_encoding` helper there to verify the message flow.

### 7. `src/serial.rs` — the heart of the data plane

600 lines, broken into clear sections by `// ----` dividers:

- L1–105: configuration enums (`DataBits`, `StopBits`, `Parity`,
  `FlowControl`) + `From` impls. Pure data.
- L106–167: `PortInfo` + `list_available()` + helpers for hardware
  ID / description formatting.
- L169–212: **`SerialIo` trait + impl for `SerialStream`** — this is
  the key abstraction. The whole rest of the file is generic over
  `Box<dyn SerialIo>`.
  - Confirm the 4 trait methods (poll-based AsyncRead/AsyncWrite
    inherited + clear_os_buffers + set_dtr_rts + set_break_state)
    are everything the tools need.
- L214–352: `SerialConnection` — open, write, read (with optional
  timeout), flush_buffers, set_dtr_rts, send_break.
  - `read()` honours `Option<u64>` for timeout, returns
    `SerialError::ReadTimeout`. The `wait_for` tool in handler.rs
    builds on this.
  - `send_break()` drops the mutex around the `tokio::time::sleep`
    so other ops can interleave.
- L354–425: `ConnectionManager` — `open`, `close`, `get`, `insert`,
  `list_open`. The `insert` method is what lets integration tests
  inject a loopback.
- L429–441: `FlushTarget` enum (`Input`/`Output`/`Both`) + `From`
  for `ClearBuffer`.
- L443–507: **`test_support` module** — `LoopbackIo` wraps a
  `DuplexStream` and stubs the control-line methods as no-ops.
  This is the in-memory backend used by the unit tests + Layer 2
  integration tests.
- L509–600: unit tests.

**Watch for:**
- The `MAX_BAUD_RATE = 4_000_000` cap. Real hardware can go higher
  in special cases (e.g. some FTDI chips at 12 Mbaud). Is the cap
  defensible or arbitrary?
- `is_port_in_use` linear scan over open connections. Fine for
  N≤10ish. If you ever expect dozens of open ports, becomes O(N) per
  open — easy fix with a second HashMap if needed.
- `send_break` drops + re-acquires the mutex between assert/release.
  Race window: another tool can grab the mutex and write garbage
  during the break. Question: is that acceptable or should send_break
  hold the lock?

### 8. `src/handler.rs` — the MCP surface (the big one)

1342 lines. Worth budgeting 30 min for this file alone. Sections in
order of appearance:

- L1–35: imports + `DEFAULT_READ_TIMEOUT_MS`.
- L37–127: **All tool argument structs** (`OpenArgs`, `WriteArgs`,
  `ReadArgs`, `FlushArgs`, `SetDtrRtsArgs`, `SendBreakArgs`,
  `WaitForArgs`, `SubscribeArgs`, `UnsubscribeArgs`, `CloseArgs`).
  All `#[derive(Deserialize, JsonSchema)]`. Defaults via small fns.
- L129–164: `default_*` helper fns. Boring but necessary for serde.
- L166–257: **All tool response structs** (`ListPortsResult`,
  `OpenResult`, `CloseResult`, `WriteResult`, `ReadResult`,
  `FlushResult`, `SetDtrRtsResult`, `SendBreakResult`,
  `SubscribeResult`, `UnsubscribeResult`, `WaitForResult`). These
  are what MCP clients see as structured output — review carefully,
  this is the API contract.
- L259–283: `SerialHandler` struct + `StreamHandle` Drop wrapper
  for background streams.
- L285–296: `SerialHandler::new()` + `with_manager()` constructor.
  The `with_manager` constructor is the test hook.
- L298 onwards: **The 11 tool methods** annotated with `#[tool]`.
  Read them in this order for understanding:
  1. `list_ports` — simplest, just enumerates.
  2. `open` — opens a port. Note `parse_open_args` does strict
     parsing (no silent fallback).
  3. `close` — removes from manager.
  4. `write` — encoding parse, decode, call `connection.write`.
  5. `read` — encoding parse, call `read_bytes` helper,
     `build_read_result` formats the response with `timed_out` flag.
  6. `flush` / `set_dtr_rts` / `send_break` — control-line ops.
  7. `wait_for` — the most algorithmic tool. Read
     `read_until_pattern` carefully.
  8. `subscribe` / `unsubscribe` — spawn a background task that
     forwards bytes as `notifications/message` events.
- After the tools: `lookup_connection` helper + `ReadOutcome` /
  `WaitOutcome` types + the pure formatting helpers.
- L~960 onwards: parsing helpers (`parse_open_args`,
  `parse_data_bits`, etc.) — these enforce the strict-arg policy.
- L~1020: tiny error builders (`log_tool_err`).
- L~1040: `#[prompt_router]` block with `diagnose_port` and
  `interactive_terminal` prompt definitions. Read the user-message
  strings — these go into the LLM context window every time a client
  calls `get_prompt`.
- L~1180: `ServerHandler` impl with `#[tool_handler]`,
  `#[prompt_handler]`, `#[task_handler]` macros stacked.
  `get_info()` declares capabilities (tools, resources, prompts,
  logging) and protocol version.
- L~1230: `list_resources` / `list_resource_templates` /
  `read_resource` methods + the `parse_resource_uri` dispatch helper.
- L~1290: tests module (15 tests).

**Watch for:**
- The 11 tools all follow a similar shape: `parse args → lookup
  connection → call SerialConnection method → format response`. Spot
  any drift between them.
- `subscribe` task: the inner loop swallows codec encoding errors
  (`Err(_) => continue`). Question: should it surface those as a
  warning notification instead of silently dropping the chunk?
- `send_break` is marked `execution(task_support = "optional")` —
  but it sleeps for a fixed duration. Cancellation while sleeping
  works because tokio cancels the future at the next await point.
  Verify you find that behaviour acceptable.
- The prompt strings (`diagnose_port`, `interactive_terminal`) are
  hand-written prose. They contain newlines + tool-call syntax. They
  go into every LLM that triggers `prompts/get`, so wording matters.
  Read them as if you were the LLM.
- `list_resources` returns the two static resources but does NOT
  declare `resources/list_changed` capability. So clients won't be
  notified when connections are opened/closed. Is that fine, or do
  you want to wire it up?

### 9. `tests/common/mod.rs` — test harness

213 lines. Read before the individual test files.

- `TestServer` lifecycle: `start_with(manager)` binds port 0, spawns
  axum::serve in a tokio task, drop guard cancels.
- `NotificationCollector` — `ClientHandler` impl that forwards
  `notifications/message` onto an mpsc.
- `connect_client()` — initialises an rmcp HTTP client via
  reqwest backend.
- `pty::PtyPair` (Unix only) — `openpty()` + raw-mode termios + path
  resolution via `ttyname`.

**Watch for:** `_slave: OwnedFd` field on `PtyPair` exists only to
keep the slave fd alive (kernel reclaim race). Confirm the comment
above it is clear enough.

### 10. `tests/http_integration.rs` — Layer 2

331 lines, 14 tests. Read at least the first three (`initialize_handshake`,
`list_tools_returns_all_eleven_tools`, `subscribe_then_peer_write_pushes_notification`)
to understand the harness pattern. Skim the rest.

### 11. `tests/serial_pty.rs` — Layer 3 (the killer test layer)

217 lines, 6 tests. **This is the file that proves "server ↔ serial
↔ client really works".** Read it top-to-bottom.

The `setup()` helper builds the full pipeline: PTY pair → server
opens slave_path → client connects via HTTP → returns
connection_id + master fd. Each test then drives one direction of
the traffic.

### 12. `tests/hardware_loopback.rs` — Layer 4

186 lines, 2 ignored tests. Glance only; they're for manual
verification with a USB-Serial dongle + TX-RX jumper. Worth knowing
they exist if you ever want to validate a hardware setup.

### 13. `CHANGELOG.md`

The history of what changed vs. the upstream `d5a8196` baseline.
Useful as a reviewer's table of contents — confirms which design
decisions came from which commit.

### 14. `.github/workflows/ci.yml`

26 lines. fmt-check + build + test + clippy on push + PR. Confirm
the order matches what you want gated on (especially: do you want
`fmt --check` to be a hard fail or a warning?).

## Cross-cutting design decisions worth a second look

These are the calls that aren't obvious from reading any one file.

1. **`SerialHandler::new()` per HTTP session.** Each connecting
   client gets its own `ConnectionManager`. Pro: isolation. Con: two
   simultaneous HTTP clients can both call `open` on the same
   physical port (the second `open_native_async` will fail at the
   OS layer, but the failure mode is "Resource busy" rather than the
   nicer "Connection already exists"). Decide if you want a shared
   manager across sessions.

2. **`Box<dyn SerialIo>` trait object.** Production wraps a
   `SerialStream`, tests wrap a `DuplexStream`. The trait is
   intentionally narrow (just 3 control-line methods + AsyncRead +
   AsyncWrite). Loopback impls return `Ok(())` from control methods,
   which is silent rather than panicking. Consider whether that's
   the right testing posture.

3. **Strict OpenArgs parsing.** Bad `data_bits="9"` now returns an
   error instead of silently defaulting to 8. Verify the LLM ergonomics
   — does the error message phrasing read well?

4. **Tool error vs McpError split.** All operational failures
   (unknown id, bad encoding, IO error during read) come back as
   `CallToolResult { isError: true }`. Only genuine protocol faults
   become `McpError`. Confirm you find that boundary defensible.

5. **Streaming uses `notifications/message` (the MCP logging
   channel), not a dedicated channel.** This works on every MCP
   client because logging is mandatory in the spec, but it conflates
   "log output" with "serial RX". An alternative is to use
   `notify_resource_updated` on `serial://connections/{id}` — read
   the comments around the `subscribe` tool and decide.

## How to verify behaviour end-to-end

After reading, exercise the code yourself:

```bash
# Unit + integration tests (fast)
cargo test
# → 36 unit + 14 HTTP + 6 PTY + 2 ignored = 56 active

# Strictest CI gate
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

# Run the HTTP server manually
SERIAL_MCP_HTTP_BIND=127.0.0.1:8000 RUST_LOG=debug \
  cargo run --release --bin serial-mcp-server-http

# Drive it with the official MCP inspector
npx @modelcontextprotocol/inspector
# → connect to http://127.0.0.1:8000/mcp
# → walk through list_tools, list_resources, get_prompt(diagnose_port)

# If you have a USB-Serial dongle with TX-RX jumpered:
SERIAL_MCP_TEST_PORT=/dev/ttyUSB0 cargo test --test hardware_loopback -- --ignored
```

## Questions to come back with

Likely candidates after you finish reading:

- Should the HTTP binary share a single `ConnectionManager` across
  sessions, or stay session-isolated?
- Streaming via `notifications/message` vs `resources/updated` — pick
  one or expose both?
- Resource subscription (`resources/list_changed` +
  `resources/updated`) — wire it up now or defer?
- Cap on simultaneous `subscribe` tasks per connection? (currently
  unbounded, though each connection is locked so only one runs at a
  time per port.)
- Prompt template wording — pass to a real LLM for a sanity read?
- Anything you want renamed, restructured, or commented further?

## Reviewer's punch list

- [ ] `Cargo.toml` deps and features
- [ ] `src/lib.rs` + `src/main.rs` + `src/bin/http.rs`
- [ ] `src/error.rs`
- [ ] `src/codec.rs`
- [ ] `src/serial.rs` (config enums → SerialIo → SerialConnection → ConnectionManager → test_support)
- [ ] `src/handler.rs` (args → results → tools → helpers → prompts → ServerHandler impl → resources)
- [ ] `tests/common/mod.rs`
- [ ] `tests/http_integration.rs`
- [ ] `tests/serial_pty.rs`
- [ ] `CHANGELOG.md`
- [ ] `.github/workflows/ci.yml`
- [ ] Run `cargo test` + clippy + fmt-check
- [ ] (Optional) Boot the HTTP server, exercise it with the MCP inspector
- [ ] (Optional) Run the hardware-loopback suite with a USB-Serial dongle
