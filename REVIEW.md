# Code Review Walkthrough — `serial-mcp-server`

> ⚠️ **Pegged to v0.2.1.** Newer files and v0.3.0 changes (single binary,
> CLI args replacing env vars) are not reflected here.
> See CHANGELOG.md for additions since v0.2.2.

## Context

Reading guide for the v0.2.1 release, now that the MCP 2025-11-25 compliance
sprint is done. This document is the reading
order plus per-file checkpoints — the things actually worth your
attention vs. the things that are just plumbing.

State of the repo, head of `main`:

```
src/
  main.rs           ~130 LOC  single entry point; --transport selects stdio or HTTP
  lib.rs              7 LOC   module declarations + re-exports
  error.rs           27 LOC   single SerialError enum
  codec.rs          184 LOC   Encoding enum + decode/encode + 9 tests
  serial.rs         600 LOC   PortInfo, SerialIo trait, ConnectionManager
                              + LoopbackIo test backend + 12 tests
  server.rs           570 LOC  11 tools, 3 resources, 2 prompts,
                              resource change notifications, port allowlist,
                              pagination, resource metadata, 15 tests
  tools/
    mod.rs           ~40 LOC   Re-exports + outputSchema verification test
    types.rs        ~200 LOC   Tool argument + response structs
    helpers.rs      ~150 LOC   read_bytes, read_until_pattern, build_read_result
    port_ops.rs      ~80 LOC   list_ports, open, close
    io_ops.rs       ~100 LOC   read, write, flush
    control_ops.rs   ~60 LOC   set_dtr_rts, send_break
    pattern_ops.rs  ~120 LOC   wait_for
    stream_ops.rs   ~150 LOC   subscribe, unsubscribe, stream_rx
  prompts/
    mod.rs            ~5 LOC   Re-exports
    types.rs         ~30 LOC   Prompt argument structs
    diagnose.rs      ~60 LOC   diagnose_port prompt
    interactive.rs   ~50 LOC   interactive_terminal prompt
  resources/
    mod.rs           ~20 LOC   Re-exports + URI constants
    types.rs         ~40 LOC   ResourceUriKind, parse_resource_uri
  security.rs        ~60 LOC   Allowlist: parse, check, summary

tests/
  common/mod.rs     213 LOC   TestServer harness + NotificationCollector
                              + PtyPair helper
  http_integration  ~380 LOC  Layer 2 — 17 in-memory HTTP tests (incl. pagination)
  resource_subscriptions ~150 LOC  Layer 3 — 2 resource subscription tests
  serial_pty        217 LOC   Layer 4 — 6 real-PTY tests (Linux only)
  hardware_loopback 186 LOC   Layer 5 — 2 ignored hardware tests
  stdio_integration ~170 LOC  Layer 6 — 3 stdio transport tests
  allowlist         ~220 LOC  Layer 7 — 3 port allowlist tests
```

Total ~3.5 KLOC production+tests. Active: 70 tests pass (plus 2 ignored hardware tests), clippy
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
 `server::*`) is reachable via path.

### 3. `src/main.rs` — stdio entry

28 lines. Single tokio::main, logging setup, `SerialHandler::new()`
on stdio.

**Watch for (v0.3.0+):** transport selected via `--transport=http`. Allowlist
via `--allowlist=<glob,...>`. Bind address via `--bind=<addr>`. `RUST_LOG`
still controls log level. HTTP path: Ctrl-C → cancellation token → graceful
drain. No TLS/auth — default bind `127.0.0.1:8000` is the safety boundary.

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
- `Encoding::from_str` returns `CodecError`; in `server.rs` callers
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
    `SerialError::ReadTimeout`. The `wait_for` tool in server.rs
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

### 8. `src/server.rs` — the MCP surface (the big one)

  570 lines. The modular refactored version. Sections in
  order of appearance:

- L1–35: imports + pagination helper.
- L37–65: `SerialHandler` struct (streams, security, subscribers).
- L67–86: `SerialHandler::new()` + `with_manager()` constructor.
- L88 onwards: **The 11 tool methods** annotated with `#[tool]`.
  Read them in this order for understanding:
  1. `list_ports` — simplest, just enumerates.
  2. `open` — opens a port. Note strict parsing (no silent fallback).
  3. `close` — removes from manager.
  4. `write` — encoding parse, decode, call `connection.write`.
  5. `read` — encoding parse, call `read_bytes` helper.
  6. `flush` / `set_dtr_rts` / `send_break` — control-line ops.
  7. `wait_for` — the most algorithmic tool.
  8. `subscribe` / `unsubscribe` — spawn background task.
- L~300: `#[prompt_router]` block with `diagnose_port` and
  `interactive_terminal` prompt definitions.
- L~340: `ServerHandler` impl with `#[tool_handler]`,
  `#[prompt_handler]`. `get_info()` declares capabilities.
- L~360: `list_resources` / `list_resource_templates` /
  `read_resource` — now with pagination and metadata.
- L~420: tests module (15 tests).

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
- `list_resources` now declares `resources/list_changed` capability
  and fires notifications on open/close. The allowlist check happens
  in the `open` tool before the connection is created.

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

~426 lines, 17 tests. Read at least the first three (`initialize_handshake`,
`list_tools_returns_all_eleven_tools`, `subscribe_then_peer_write_pushes_notification`)
to understand the harness pattern. Skim the rest.

### 11. `tests/serial_pty.rs` — Layer 4 (the killer test layer)

217 lines, 6 tests. **This is the file that proves "server ↔ serial
↔ client really works".** Read it top-to-bottom.

The `setup()` helper builds the full pipeline: PTY pair → server
opens slave_path → client connects via HTTP → returns
connection_id + master fd. Each test then drives one direction of
the traffic.

### 12. `tests/resource_subscriptions.rs` — Layer 5 (resource subscription tests)

~95 lines, 2 tests. Verifies resource subscribe/unsubscribe roundtrip
and actual subscription behavior with notification delivery.

### 13. `tests/stdio_integration.rs` — Layer 6 (stdio transport)

~217 lines, 4 tests (1 ignored). Spawns the stdio binary as a child
process and connects via stdin/stdout pipes using rmcp's
`TokioChildProcess` transport. Verifies the stdio transport works
identically to HTTP — critical for desktop MCP clients (Claude Desktop,
opencode).

### 14. `tests/hardware_loopback.rs` — Layer 7 (hardware validation)

186 lines, 2 ignored tests. Real USB-Serial adapter with TX→RX jumper.
Confirms behaviour on physical hardware. Run with:
`SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored --test-threads=1`

### 15. `tests/allowlist.rs` — Layer 8 (security)

~220 lines, 3 tests. Tests the port allowlist (passed via `--allowlist`):
blocks unauthorized ports, allows authorized ports, glob patterns (`/dev/ttyACM*`).

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
# → 37 unit + 17 HTTP + 2 resource_subscriptions + 6 PTY + 3 stdio + 3 allowlist + 2 ignored hardware = 70 active
```

## Questions to come back with

Likely candidates after you finish reading:

- Should the HTTP binary share a single `ConnectionManager` across
  sessions, or stay session-isolated? (Current decision: isolated for safety)
- Cap on simultaneous `subscribe` tasks per connection? (currently
  unbounded, though each connection is locked so only one runs at a
  time per port.)
- Prompt template wording — pass to a real LLM for a sanity read?
- Progress notifications for long-running tools — wire up now or defer?
- Completions for tool arguments — auto-complete ports, baud rates?
- Anything you want renamed, restructured, or commented further?

## Reviewer's punch list

- [ ] `Cargo.toml` deps and features
- [ ] `src/lib.rs` + `src/main.rs`
- [ ] `src/error.rs`
- [ ] `src/codec.rs`
- [ ] `src/serial.rs` (config enums → SerialIo → SerialConnection → ConnectionManager → test_support)
- [ ] `src/server.rs` (args → results → tools → helpers → prompts → ServerHandler impl → resources)
- [ ] `tests/common/mod.rs`
- [ ] `tests/http_integration.rs`
- [ ] `tests/serial_pty.rs`
- [ ] `tests/stdio_integration.rs`
- [ ] `tests/allowlist.rs`
- [ ] `tests/hardware_loopback.rs`
- [ ] `CHANGELOG.md`
- [ ] `.github/workflows/ci.yml`
- [ ] Run `cargo test` + clippy + fmt-check
- [ ] (Optional) Boot the HTTP server, exercise it with the MCP inspector
- [ ] (Optional) Run the hardware-loopback suite with a USB-Serial dongle
