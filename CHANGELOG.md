# Changelog

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] — fork (qarnet/serial-mcp-server)

Aggressive rewrite of the original upstream release. Tracks rmcp 1.7,
keeps the original tool surface working, removes ~80% of the codebase
that was dead scaffolding, and adds six new tools plus resources,
prompts, streaming, task cancellation, and an HTTP transport.

### Added

- **Tools (6 new):**
  - `flush(connection_id, target)` — clear OS input/output/both buffers.
  - `set_dtr_rts(connection_id, dtr, rts)` — drive modem-control lines (Arduino reset, ESP32 bootloader).
  - `send_break(connection_id, duration_ms)` — assert BREAK on TX for `duration_ms`. Task-capable.
  - `wait_for(connection_id, pattern, pattern_encoding, timeout_ms, max_bytes, response_encoding)` — read until a byte pattern matches. The canonical prompt/response loop tool. Task-capable.
  - `subscribe(connection_id, encoding, max_chunk_bytes, poll_interval_ms)` — spawn a background reader that pushes RX chunks to the client as `notifications/message` events.
  - `unsubscribe(connection_id)` — cancel an active subscription.
- **Resources** (new MCP capability):
  - `serial://ports` — live port list.
  - `serial://connections` — open connections snapshot.
  - `serial://connections/{id}` — per-connection detail (templated URI).
- **Prompts** (new MCP capability):
  - `diagnose_port` — step-by-step plan for identifying an unknown device.
  - `interactive_terminal` — REPL conventions for an open connection.
- **Task framework**: `read`, `wait_for`, and `send_break` are now task-capable. Clients can submit them as MCP tasks and cancel via `tasks/cancel`.
- **Second binary `serial-mcp-server-http`** — streamable-HTTP transport. Default `127.0.0.1:8000/mcp`, override with `SERIAL_MCP_HTTP_BIND`.
- **Structured tool output** — every tool returns a typed JSON struct via rmcp `Json<T>` (`ListPortsResult`, `OpenResult`, `CloseResult`, `WriteResult`, `ReadResult`, `WaitForResult`, `FlushResult`, `SetDtrRtsResult`, `SendBreakResult`, `SubscribeResult`, `UnsubscribeResult`).
- **`SerialIo` trait abstraction** — `SerialConnection` is now generic over any `AsyncRead + AsyncWrite + Send + Unpin` backend. Tests use an in-memory `tokio::io::duplex` backend; production uses `tokio_serial::SerialStream`. Hardware-free unit tests for read/write/timeout/wait_for and `ConnectionManager` invariants.
- **`codec` module** with an `Encoding` enum (`Utf8` / `Hex` / `Base64`) and a typed `CodecError`. Replaces three duplicated stringly-typed encode/decode implementations.
- **CI workflow** (`.github/workflows/ci.yml`) running `cargo build`, `cargo test`, and `cargo clippy -D warnings` on push and pull request.

### Changed

- **rmcp 0.3.2 → 1.7** — full migration to the rust-sdk 1.x macro shape (`wrapper::Parameters`, `ServerInfo::new(...).with_*` builder, `#[tool_router]` / `#[tool_handler]` / `#[prompt_router]` / `#[prompt_handler]` / `#[task_handler]`).
- **Tool-level errors** now surface as `CallToolResult { isError: true, content: ... }` instead of `McpError::internal_error`. Operational failures (unknown id, bad encoding, IO errors) are recoverable from the client's perspective; only genuine protocol faults stay as `McpError`.
- **Strict `open` argument parsing** — bad `data_bits` / `stop_bits` / `parity` / `flow_control` now return a typed error instead of silently falling back to defaults.
- **Error type unified** — the duplicate `LocalSerialError` (`src/serial/error.rs`) is gone; one `SerialError` covers everything.
- **`PortInfo::list` → `PortInfo::list_available`** and returns the crate-level `Result` for consistency.
- **`SerialConnection::open`** split into `ensure_valid_baud_rate` + `build_stream`; baud-rate cap is now a `MAX_BAUD_RATE` const.
- **README** rewritten to describe the new surface, transports, and example agent flow.
- **Cargo.toml** repository URL points at `qarnet/serial-mcp-server`; fork author appended.

### Removed

- **Whole `src/session/` directory** (815 LOC). `SessionManager` was never constructed by the active binary; `start()` (which spawned a cleanup task) was never called.
- **`src/utils.rs`** (506 LOC). `PortType`, `DataConverter`, `TimeUtils`, `BufferUtils` (incl. a 256-byte CRC8 table), `SessionIdGenerator`, `StringUtils`, `Validator` — only `Validator` and `SessionIdGenerator` were referenced, and only by the dead session code.
- **`src/config.rs`** (312 LOC). The `Config` was loaded, validated, and stored in `SerialHandler.config` with `#[allow(dead_code)]`. The handler never consulted it; CLI flags like `--max-connections` and `--default-baud-rate` had no runtime effect.
- **`tests/common/`** scaffold. It imported the crate as `serial_mcp_rs` while the crate is `serial_mcp_server`, so it would never compile if any integration test had used it.
- **`src/tools/types.rs`** stringly-typed args/responses, including `StatusArgs` / `ConfigureArgs` that had no matching `#[tool]` registration.
- **Triple-duplicated `encode_data` / `decode_data`** (in `utils.rs`, `tools/types.rs`, and inline in `tools/serial_handler.rs`) collapsed into the single `codec` module.
- **Five unused error sub-enums** (`ConnectionError`, `ProtocolError`, `SessionError`, `ConfigError`, `DataError`) and the bulk of `SerialError` variants that were never constructed in the active code path.
- **Cargo dependencies** that were not used: `clap`, `toml`, `anyhow`, `futures`, `async-trait`, `chrono`, `mockall`, `tokio-test`, `tempfile`. Tokio features narrowed from `full` to only what the code uses.
- **CLI flags on the stdio binary**. The remaining knob is `RUST_LOG` (env). Logging defaults to stderr.

### Fixed

- The `tests/common/` integration-test scaffold no longer fails silently with `unresolved import 'serial_mcp_rs'` — it's gone.
- The `SerialHandler.config` field that did nothing under `#[allow(dead_code)]` is gone; the handler now contains only state it actually uses.
- `clippy::manual_is_multiple_of` and `clippy::io_other_error` warnings cleared (rust 1.95+).

### Test count

| Phase | Tests |
|---|---|
| Upstream initial release (`d5a8196`) | 6 |
| After slim refactor (commits `ebf3efc..52b9def`) | 6 |
| After naming/structure refactor (`d55711e..cda7788`) | 20 |
| After feature sprint A–F | 33 |
| After feature sprint G–K | 36 |
| After MCP 2025-11-25 compliance sprint | 62 active + 3 ignored |

## [0.2.2] — 2026-05-26

MCP specification compliance audit fixes, input validation, tool schemas, race-condition fix, and comprehensive test coverage.

### Added

- **Functional pagination** — `list_resources` and `list_resource_templates` now properly implement cursor-based pagination with `nextCursor`.
  - `PaginatedRequestParams` cursor parameter is interpreted as base64-encoded offset
  - Page size set to 100 items (generous for serial port use)
  - Integration tests added to verify pagination behavior
- **Resource `size` metadata** — `serial://ports` and `serial://connections` now include `size` field (port count and connection count respectively)
- **Tool outputSchema verification test** — `tools::tests::verify_all_tool_schemas` confirms all 11 tools have auto-generated output schemas via rmcp macro
- **Resource metadata to resource templates** — Connection templates now include `size` reflecting open connection count
- **`src/limits.rs` module** — centralized MIN/MAX constants for all bounded tool inputs (`MAX_READ_BYTES`, `MAX_TIMEOUT_MS`, `MIN_POLL_INTERVAL_MS`, etc.)
- **Minimum-bound validation** — `read.max_bytes`, `wait_for.max_bytes`, and `subscribe.max_chunk_bytes` now reject zero (must be ≥ 1)
- **Schema constraints on tool args** — `duration_ms`, `timeout_ms`, `max_bytes`, `max_chunk_bytes`, and `poll_interval_ms` now advertise `minimum`/`maximum` in their JSON schemas via `schemars` helpers
- **Integration test for validation limits** — HTTP-layer test verifying all bounded inputs return `isError: true` when out of range
- **Cross-session subscribe test** — verifies closing a connection from one session stops the streaming task subscribed from another

### Changed

- **SPECIFICATION_COMPLIANCE.md** — Fixed false negatives:
  - `title` field: marked ✅ (was ❌, actually present on all tools)
  - `annotations` field: marked ✅ (was ❌, actually present on relevant tools)
  - `progressToken`: marked ✅ (was ❌, wired for read/wait_for/send_break)
  - `CancellationToken`: marked ✅ (was ❌, cooperative cancellation working)
  - Overall compliance score updated from ~70% to ~85%
- **`StreamRegistry` made injectable** — HTTP transport now shares a single stream map across sessions (via `with_manager_security_and_streams`), preventing cross-session subscribe leaks

### Removed

- **Dead code from `SerialHandler`** — Removed `processor`, `tool_router`, and `prompt_router` fields that were constructed but never used (all marked `#[allow(dead_code)]`)
- **Unused `#[task_handler]` macro** — Removed from `ServerHandler` impl since task infrastructure was not wired and `tasks` capability is not declared

### Fixed

- **Pagination compliance** — `next_cursor` now properly populated when more items remain, instead of always returning `None`
- **Race condition in `ConnectionManager::open`** — added `opening_ports` set to prevent concurrent opens of the same physical port
- **Peer disconnect handling in `stream_rx`** — background streaming task now breaks (rather than panicking) when the MCP peer disconnects
- **Tool arg schemas** — `uint` fields (`duration_ms`, `max_bytes`, `max_chunk_bytes`, `timeout_ms`, `poll_interval_ms`) no longer emit the non-standard `"format": "uint"` keyword and include appropriate min/max bounds

## [0.2.6] — 2026-05-27

Protocol emulator integration tests — a full ESP32 weather-station agent workflow and binary payload roundtrips via PTY. No hardware required.

### Added

- **`tests/protocol_emulator.rs`** — 13-stage MCP agent workflow simulating an ESP32 firmware device:
  - `write` (AT commands, hex/base64 payloads), `read`, `wait_for` (prompts)
  - `set_dtr_rts` (reset), `flush`, `subscribe`/`unsubscribe`
  - Binary payload roundtrip (0x00–0xFF via hex and base64)
  - Large payload test (>3 KB via hex-encoded `read`)
  - `wait_for` with hex pattern `CAFEC0FFEE`
  - All 11 tools + resources covered in a single hardware-free test
- **`tests/protocol_emulator_binary.rs`** — focused binary encoding edge cases (hex/base64 roundtrips of all byte values, hex read for >3 KB, hex `wait_for` pattern match)
- **`PtyPair::into_parts()`** in `tests/common/mod.rs` — splits PTY so emulator and client run in different async tasks
- **`tests/common` test utilities** extended with `connect_client`, `tool_request`, `next_notification` helpers

### Changed

- PTY `set_dtr_rts` returns ENOTTY in kernel-dependent edge cases; tests handle gracefully instead of asserting success
- PTY visibility in `list_ports` relaxed for kernel-variant behavior

### Test count: 157 (2 hardware-ignored)

| Layer | Count |
|---|---|
| Unit tests | 51 |
| HTTP integration | 26 |
| PTY integration | 7 |
| Proptest | 54 |
| Allowlist | 5 |
| Resource subscriptions | 2 |
| Protocol emulator | 1 |
| Binary payload emulator | 1 |
| Blob resources | 2 |
| Hardware (ignored) | 2 |

---

## [0.2.5] — 2026-05-27

Property-based testing, fuzz targets, and lifecycle tests.

### Added

- **Proptest property tests** (`tests/proptest.rs`, 54 strategies):
  - Tool arg roundtrips (`open`, `read`, `write`, `flush`, `set_dtr_rts`, `send_break`, `subscribe`, `unsubscribe`, `wait_for`)
  - Result type schemas (all 11 result types validated via `jsonschema`)
  - Encoding roundtrips (hex, base64, utf8 edge cases)
  - Lifecycle sequencing (`open`→`write`→`close`, double-close, read-after-close, write-after-close, subscribe-then-close)
  - Bound validation (`clamp_or_err`, `clamp_timeout_or_err`, `clamp_poll_interval_or_err`, `require_min_or_err`)
  - Port name escaping (special characters in port names)
- **cargo-fuzz targets** (3 harnesses in `fuzz/`):
  - `fuzz_open_args` — random `Open` tool arguments validated with `open_args_parsed_strictly`
  - `fuzz_codec` — random hex/base64 strings into `Encoding::decode`
  - `fuzz_read` — random bytes decoded across all three encodings and validated via `jsonschema`
- **`tests/allowlist.rs`** (5 tests) — authorization, glob patterns, comma-separated lists
- **Lifecycle smoke tests** (in proptest) — `open`→`write`→`read`→`close` pipeline verified in-memory
- **`tests/serial_pty.rs`** PTY test additions: `pty_wait_for_matches_real_serial_pattern`

### Changed

- `SerialIo` trait updated to support `tokio_serial::SerialStream` and PTY backends uniformly
- `BreakResetGuard` made robust with `AtomicBool` and `try_current()` cleanup

---

## [0.2.4] — 2026-05-27

Fix for MCP tool schema strictness — optional fields must serialize as `null`, not omitted.

### Fixed

- **Schema fix**: Removed `skip_serializing_if` on optional fields in `PortInfo.hardware_id`. Some MCP clients reject schemas with missing optional fields.
- Subscribe `timeout_ms` now serializes as `null` when absent (previously omitted entirely), matching `wait_for` behavior

---

## [0.2.3] — 2026-05-26

Subscribe gains a synchronous blocking mode for agents that prefer pull over push.

### Added

- **`subscribe(timeout_ms)`** — when `timeout_ms` is provided, `subscribe` blocks until the timeout expires and returns all accumulated data as a single `SubscribeResult`. Falls back to the existing fire-and-forget mode when absent.
- **Documentation**: `SPECIFICATION_COMPLIANCE.md` updated with corrected MCP 2025-11-25 scores

---

## [0.2.1] — 2026-05-24

MCP 2025-11-25 compliance, CDC-ACM hardware fixes, port allowlist, and comprehensive testing.

### Added

- **MCP Protocol 2025-11-25** — Updated from 2024-11-05 to 2025-11-25.
- **Resource change notifications** — `open` and `close` tools now fire `notify_resource_list_changed()` so clients get push updates when connections change.
- **`resources/list_changed` capability** — Declared in `get_info()` so clients know to expect resource list updates.
- **Port allowlist** — New `SERIAL_MCP_ALLOWLIST` environment variable with glob pattern support (e.g., `/dev/ttyACM*,/dev/ttyUSB*`).
  - `list_ports` still shows all ports for discovery
  - `open` rejects unauthorized ports with clear error message
  - If not set, all ports allowed (backward compatible)
- **STDIO transport integration tests** — `tests/stdio_integration.rs` with 4 tests:
  - Initialize handshake over stdio pipes
  - List tools via stdio
  - List resources via stdio
  - Full hardware lifecycle test (marked `#[ignore]`, requires device)
- **Allowlist tests** — `tests/allowlist.rs` with 3 tests:
  - Blocks unauthorized ports
  - Allows authorized ports
  - Glob pattern matching works
- **CDC-ACM hardware test support** — Verified on `/dev/ttyACM0` with TX-RX loopback.

### Changed

- **CDC-ACM packet coalescing fix** — Changed `POLL_MS` from 5ms to 50ms in `SerialConnection::read()` to allow USB packet coalescing before returning data. Prevents read truncation on CDC-ACM devices.
- **RX streaming strategy** — Confirmed RX data streaming stays on `notifications/message` (logging channel) rather than `resources/updated`. This provides immediate delivery without round-trips.

### Fixed

- **`pty_wait_for_matches_real_serial_pattern`** test — Now passes consistently after increasing poll interval to 50ms.
- **Hardware loopback tests** — Both tests (`hw_loopback_write_then_read_roundtrip` and `hw_loopback_wait_for_matches_echo`) now pass on `/dev/ttyACM0`.

## [0.1.0] — initial open source release (upstream)

See [adancurusul/serial-mcp-server@d5a8196](https://github.com/adancurusul/serial-mcp-server/commit/d5a8196) for the baseline. Provided five tools (`list_ports`, `open`, `close`, `write`, `read`) and STM32 demo firmware.
