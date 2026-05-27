# Protocol Emulator Test Plan

## Goal

Add a single integration test (`protocol_emulator_workflow`) to
`tests/protocol_emulator.rs` that simulates a full MCP agent session
against a firmware-like PTY device emulator.  No physical hardware needed.

## Overview

The test creates a PTY pair, starts the real MCP server against the slave
side, and runs a device-emulator task on the master side that responds to
commands exactly like the ESP32 weather station firmware.  The test then
drives all 11 MCP tools through the HTTP client in a realistic multi-step
workflow — exactly the pattern an AI agent uses.

### Architecture

```
┌──────────────┐   HTTP    ┌──────────────┐  PTY slave   ┌─────────────────┐
│  MCP client  │◄─────────►│ MCP server   │◄────────────►│ Device emulator  │
│  (test code) │  tools/   │ (axum HTTP)  │tokio_serial  │ (tokio task)    │
│              │  resources│              │              │ Reads commands,  │
└──────────────┘           └──────────────┘              │ writes responses │
                                                         └─────────────────┘
```

### Device emulator specification

The emulator must implement the ESP32 weather-station serial protocol
exactly as specified in:
- `~/repos/MC-ESP-Klimastation/AGENTS.md` lines 539–561
- `~/repos/serial-mcp-server/tests/common/mod.rs` lines 225–278 (PTY utilities)

Commands:
```
READ KV\r\n    → D=26.05.2026T23:19:02 T=26.75 H=53.30 P=980.9 C=409 V=1\r\n
READ CSV\r\n   → 26.05.2026T23:19:02;26.75;53.30;980.9;409;1\r\n
READ FL\r\n    → 26.05.2026T23:19:02  26.75  53.30  980.9   409    1\r\n
READ\r\n       → same as KV (default format)
READ   KV\r\n  → same as KV (extra whitespace — firmware strips it)
READ GARBAGE\r\n → no response (firmware ignores invalid format)
<garbage>\r\n  → no response (firmware checks line starts with READ)
```

Behavioral details (from `~/repos/MC-ESP-Klimastation/src/Interaction/UsbSerial/SerialReceiver.cpp`):
- Lines terminated by `\n`, with optional `\r` stripped.
- Requires at least 4 characters starting with `READ`.
- Skips an optional space after `READ` before parsing format.
- KV, CSV, FL are the three recognized formats.
- Empty format string uses the default (KV).
- Invalid formats are silently ignored (returns false from parseLine, no response).

Emulator task pseudocode:
```rust
async fn emulator_task(mut pty: PtyPair) {
    let mut buf = vec![0u8; 256];
    let mut pos = 0usize;
    loop {
        let n = pty.master.read(&mut buf[pos..]).await?;
        if n == 0 { break; }  // PTY closed
        pos += n;
        // Process complete lines
        while let Some(line_end) = buf[..pos].iter().position(|&b| b == b'\n') {
            let mut line = buf[..line_end].to_vec();
            // Strip trailing \r
            if line.last() == Some(&b'\r') { line.pop(); }
            let response = match parse_command(&line) {
                Some(fmt) => format_response(fmt, get_sensor_values()),
                None => continue,  // unrecognized — no response
            };
            pty.master.write_all(response.as_bytes()).await?;
            // Remove processed bytes
            let consumed = line_end + 1;
            buf.copy_within(consumed..pos, 0);
            pos -= consumed;
        }
    }
}
```

Sensor values returned (use static values — the test asserts on them):
```
Date:   26.05.2026 23:19:02
Temp:   26.75
Hum:    53.30
Press:  980.9
eCO2:   409
VCC:    1
```

Format responses:

| Format | Template | Raw bytes |
|--------|----------|-----------|
| KV | `D=26.05.2026T23:19:02 T=26.75 H=53.30 P=980.9 C=409 V=1` | `D=26.05.2026T23:19:02 T=26.75 H=53.30 P=980.9 C=409 V=1\r\n` |
| CSV | `26.05.2026T23:19:02;26.75;53.30;980.9;409;1` | `26.05.2026T23:19:02;26.75;53.30;980.9;409;1\r\n` |
| FL | `26.05.2026T23:19:02  26.75  53.30  980.9   409    1` | `26.05.2026T23:19:02  26.75  53.30  980.9   409    1\r\n` |

## MCP tool workflow

The test uses the existing HTTP test harness:
- `TestServer::start()` from `tests/common/mod.rs` line 49
- `connect_client()` from `tests/common/mod.rs` line 132
- `tool_request()` / `args_object()` from `tests/common/mod.rs` lines 195–205
- `PtyPair::open()` from `tests/common/mod.rs` line 247
- `next_notification()` from `tests/common/mod.rs` line 208

### Workflow stages:

**Stage 0: Set up**
1. Open `PtyPair` (creates `/dev/pts/N` master+slave).
2. Spawn the device emulator task on the PTY master.
3. Start `TestServer` and connect `rmcp` HTTP client.
4. Call `open` tool on the PTY slave path at 115200 baud (baud rate is ignored
   on PTY, pick any valid value).  Extract `connection_id` from result.

**Stage 1: list_ports**
- Call `list_ports` tool.
- Assert `count >= 1` and the response includes a port with `name` matching
  the PTY slave path.

**Stage 2: write + subscribe (blocking mode) ← the core feature**
- Call `flush(connection_id, target="input")` to discard any startup noise.
- Call `write(connection_id, data="READ KV\r\n", encoding="utf8")`.
  Assert `bytes_written >= 9`.
- Call `subscribe(connection_id, timeout_ms=3000, encoding="utf8")`.
  Assert `isError == false`, `data` is not null.
- Assert `data` contains `T=26.75`, `H=53.30`, `P=980.9`, `C=409`.
- Assert `bytes_read > 0`, `elapsed_ms > 0`, `timeout_ms == 3000`.

**Stage 3: write + read (standard read)**
- Call `flush(connection_id, target="input")`.
- Call `write(connection_id, data="READ CSV\r\n", encoding="utf8")`.
- Call `read(connection_id, timeout_ms=2000, encoding="utf8")`.
- Assert `data` contains `26.75;53.30;980.9;409` (semicolon-separated).
- Assert `elapsed_ms` is present.

**Stage 4: write hex-encoded command + read hex response**
- Call `flush(connection_id, target="input")`.
- Call `write(connection_id, data="52 45 41 44 20 4b 56 0d 0a", encoding="hex")`
  (this is "READ KV\r\n" in hex).
- Call `read(connection_id, timeout_ms=2000, encoding="hex")`.
- Assert response decoded back to UTF-8 contains `T=26.75`.

**Stage 5: wait_for (pattern match)**
- Call `flush(connection_id, target="input")`.
- Spawn background write of `READ KV\r\n` (100ms delay), then:
- Call `wait_for(connection_id, pattern="T=", timeout_ms=5000, max_bytes=1024)`.
- Assert `matched == true`, `match_index` is `Some`.
- Assert `data` contains `T=26.75`.

**Stage 6: wait_for timeout (no data scenario)**
- Call `flush(connection_id, target="input")`.
- Call `wait_for(connection_id, pattern="IMPOSSIBLE", timeout_ms=100, max_bytes=64)`.
- Assert `isError == true`, text mentions "timed out".

**Stage 7: subscribe fire-and-forget + notifications**
- Call `subscribe(connection_id, poll_interval_ms=50)` (no `timeout_ms`).
- Assert `data` is `null`, `bytes_read` is `null` — fire-and-forget mode.
- Call `write(connection_id, data="READ KV\r\n", encoding="utf8")`.
- Use `next_notification(rx, Duration::from_secs(2))` to capture the
  MCP logging event that the background streamer emits.
- Assert the notification's `data["data"]` contains `T=26.75`.

**Stage 8: read timeout**
- Call `flush(connection_id, target="input")`.
- Write an invalid command: `write(connection_id, data="READ GARBAGE\r\n")`.
  The emulator will not respond to this.
- Call `read(connection_id, timeout_ms=300, max_bytes=64)`.
- Assert `isError == true`, error mentions "timed out".

**Stage 9: subscribe blocking empty data (valid command, fast timeout)**
- Call `flush(connection_id, target="input")`.
- Call `subscribe(connection_id, timeout_ms=300, encoding="utf8")`.
  No command was written yet, so the emulator hasn't sent anything.
- Assert `isError == false` (subscribe doesn't error on empty read — it
  just returns whatever it collected, which may be empty).
- Assert `bytes_read` is 0 (or absent / null).

**Stage 10: flushes, set_dtr_rts, send_break, unsubscribe**
- Call `flush(connection_id, target="output")` — assert success.
- Call `flush(connection_id, target="both")` — assert success.
- Call `set_dtr_rts(connection_id, dtr=true, rts=false)` — assert `dtr` and `rts`
  fields match.
- Call `send_break(connection_id, duration_ms=30)` — assert `actual_duration_ms`
  is present and `>= 30`.
- Call `subscribe(connection_id, poll_interval_ms=50)` — subscribe again.
- Call `unsubscribe(connection_id)` — assert `was_active == true`.
- Call `unsubscribe(connection_id)` again — assert `was_active == false`.

**Stage 11: resources**
- Call `read_resource("serial://ports")` — assert JSON content includes the
  PTY slave path.
- Call `read_resource("serial://connections")` — assert JSON content includes
  the connection_id.
- Call `read_resource("serial://connections/{connection_id}")` — assert JSON
  includes `port` field matching slave path.

**Stage 12: close**
- Call `close(connection_id)` — assert success.
- Call `read(connection_id, ...)` — assert `isError == true` (closed conn).

**Stage 13: cleanup**
- `client.cancel().await.ok()` plus TestServer `Drop` plus PtyPair `Drop`
  handles all cleanup.

## Implementation details

### File to create

`tests/protocol_emulator.rs` (name must match the convention of other test
files — this will be discovered by `cargo test` automatically).

### Imports needed

All from `tests/common/mod.rs`:
- `TestServer`, `connect_client`, `tool_request`, `args_object`, `next_notification`
- `PtyPair` (under `common::pty`)

From `rmcp::model`:
- `CallToolRequestParams`, `ReadResourceRequestParams`

From `serde_json`:
- `json`

### Important: don't use `#[cfg(target_os = "linux")]`

The test file should have the standard `#![cfg(target_os = "linux")]` attribute
at the top since it uses `openpty(3)`.

### The emulator task

The emulator must be spawned as a `tokio::spawn` before the server starts.
It takes ownership of the `PtyPair` and runs for the lifetime of the test.
The task ends when the PTY master is dropped (read returns 0).

Don't leak the task — the `PtyPair` owns the master fd, so when the test
function returns and drops the TestServer, the PtyPair is dropped, the
master fd closes, and the emulator task's `read` returns 0, terminating
naturally.

The emulator uses `tokio::fs::File` reads (from the PTY master's tokio
File handle from `PtyPair`).  Since `PtyPair.master` is private, the
emulator will need a helper:

```rust
// In tests/common/mod.rs, add to impl PtyPair:
pub fn into_master(self) -> tokio::fs::File {
    self.master
}
```

Actually, the existing `PtyPair::write_device` and `read_device` are
sufficient.  But for the emulator task's blocking read loop, we need
direct access to the master.  The cleanest approach is to move the master
out of the PtyPair and drop the rest.

**Better approach: split the PtyPair**

```rust
async fn emulator_task(mut master: tokio::fs::File) {
    // ... read loop ...
}

// In the test:
let pty = PtyPair::open().expect("openpty");
let slave_path = pty.slave_path.to_string_lossy().into_owned();
let (master, slave_fd) = pty.into_parts();  // splits into master File + OwnedFd
tokio::spawn(emulator_task(master));

// Keep slave_fd alive (it's the _slave field from PtyPair)
```

This needs a new method on PtyPair:

```rust
pub fn into_parts(self) -> (tokio::fs::File, OwnedFd) {
    (self.master, self._slave)
}
```

Add this to `tests/common/mod.rs` in the `impl PtyPair` block, between
`read_device_exact` and the closing `}`.

### Response format helper

```rust
struct SensorSnapshot {
    date: &'static str,  // "26.05.2026T23:19:02"
    temp: f64,           // 26.75
    hum: f64,            // 53.30
    press: f64,          // 980.9
    co2: u32,            // 409
    vcc: u8,             // 1
}

fn format_kv(s: &SensorSnapshot) -> String {
    format!("D={} T={:.2} H={:.2} P={:.1} C={} V={}\r\n",
        s.date, s.temp, s.hum, s.press, s.co2, s.vcc)
}

fn format_csv(s: &SensorSnapshot) -> String {
    format!("{};{:.2};{:.2};{:.1};{};{}\r\n",
        s.date, s.temp, s.hum, s.press, s.co2, s.vcc)
}

fn format_fl(s: &SensorSnapshot) -> String {
    format!("{}  {:.2}  {:.2}  {:.1}   {}    {}\r\n",
        s.date, s.temp, s.hum, s.press, s.co2, s.vcc)
}
```

### Command parser

```rust
enum Format { KV, CSV, FL }

fn parse_command(line: &[u8]) -> Option<Format> {
    let line = std::str::from_utf8(line).ok()?;
    if line.len() < 4 || !line.starts_with("READ") { return None; }
    let rest = &line[4..];
    let format_str = rest.trim_start(); // skip optional spaces
    match format_str {
        "KV" => Some(Format::KV),
        "CSV" => Some(Format::CSV),
        "FL" => Some(Format::FL),
        "" => Some(Format::KV),   // default format
        _ => None,                // unknown — no response
    }
}
```

### Full emulator task

```rust
async fn emulator_task(mut master: tokio::fs::File) {
    let snapshot = SensorSnapshot {
        date: "26.05.2026T23:19:02", temp: 26.75, hum: 53.30,
        press: 980.9, co2: 409, vcc: 1,
    };
    let mut buf = vec![0u8; 256];
    let mut pos: usize = 0;
    loop {
        let n = match master.read(&mut buf[pos..]).await {
            Ok(0) | Err(_) => break,
            Ok(n) => n,
        };
        pos += n;
        while let Some(nl) = buf[..pos].iter().position(|&b| b == b'\n') {
            let line = &buf[..nl];
            let line = if line.last() == Some(&b'\r') { &line[..line.len()-1] } else { line };
            if let Some(fmt) = parse_command(line) {
                let resp = match fmt {
                    Format::KV  => format_kv(&snapshot),
                    Format::CSV => format_csv(&snapshot),
                    Format::FL  => format_fl(&snapshot),
                };
                let _ = master.write_all(resp.as_bytes()).await;
            }
            let consumed = nl + 1;
            buf.copy_within(consumed..pos, 0);
            pos -= consumed;
        }
    }
}
```

## Existing tests to reference

- `tests/serial_pty.rs:25` — `setup()` function that opens PTY + starts server + calls `open` tool.  This is the exact pattern to follow.
- `tests/serial_pty.rs:114` — `pty_subscribe_streams_device_writes_as_notifications` — subscribe + write + notification capture pattern.
- `tests/serial_pty.rs:157` — `pty_wait_for_matches_real_serial_pattern` — wait_for usage with PTY.
- `tests/http_integration.rs:396` — `subscribe_with_timeout_collects_and_returns_data` — blocking subscribe test pattern.
- `tests/http_integration.rs:433` — `subscribe_without_timeout_is_fire_and_forget` — fire-and-forget test pattern.

## How to run

```bash
cargo test --test protocol_emulator
```

Must pass on Linux (PTY support via `nix` crate).

## What this catches that existing tests don't

1. **Multi-tool workflow**: Tests tool chaining (`open → write → subscribe → write → read → write → wait_for → close`) in a single session.
2. **Real protocol interaction**: The emulator implements the actual ESP32 firmware parsing logic, testing that the MCP server correctly handles command/response sequences.
3. **subscribe timing**: Tests that the blocking subscribe mode captures data that arrives mid-window (the key bug we fixed).
4. **Invalid command handling**: Tests that commands the device ignores produce appropriate timeouts, not hung connections.
5. **All three response formats**: KV, CSV, FL — validates each encoding path.
6. **Hex encoding roundtrip**: Sends hex-encoded commands and receives hex-encoded responses.
7. **Resource consistency**: Verifies resources list the correct port path after open.
8. **Graceful cleanup**: close → read-after-close fails, unsubscribe → re-subscribe works.
