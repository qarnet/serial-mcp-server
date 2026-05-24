# Serial MCP Server

[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://rust-lang.org)
[![RMCP](https://img.shields.io/badge/RMCP-1.7-blue.svg)](https://github.com/modelcontextprotocol/rust-sdk)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

An MCP (Model Context Protocol) server that lets AI assistants drive serial
ports: open, read, write, wait for prompts, stream RX bytes, toggle DTR/RTS,
send BREAK.

**MCP 2025-11-25 compliant** with resource change notifications, port
allowlist, and comprehensive hardware testing.

## What It Does

This server exposes serial ports as MCP tools so agents like Claude can
interact with embedded devices, Arduino boards, STM32 microcontrollers, and
any UART/USB-serial hardware — all through natural language.

**Key features:**

- **11 tools** — list_ports, open, close, read, write, flush, set_dtr_rts,
  send_break, wait_for, subscribe, unsubscribe
- **3 resources** — `serial://ports`, `serial://connections`,
  `serial://connections/{id}`
- **2 prompt templates** — `diagnose_port`, `interactive_terminal`
- **Live RX streaming** — background task pushes bytes as MCP notifications
- **Task cancellation** — cancel long-running reads/waits via `tasks/cancel`
- **Resource change notifications** — clients get push updates on open/close
- **Port allowlist** — restrict which ports can be opened
- **Two transports** — stdio (desktop clients) and streamable HTTP (remote)

## Quick Start

### Build

```bash
git clone https://github.com/qarnet/serial-mcp-server.git
cd serial-mcp-server
cargo build --release
```

On Linux install libudev first:

```bash
sudo apt install libudev-dev      # Debian/Ubuntu
sudo dnf install systemd-devel    # Fedora
```

### Run

Two binaries are produced:

| Binary | Transport | Usage |
|---|---|---|
| `serial-mcp-server` | stdio | Desktop clients (Claude Desktop, opencode) |
| `serial-mcp-server-http` | HTTP | Remote access on `127.0.0.1:8000/mcp` |

**Environment variables:**

- `RUST_LOG=debug` — verbose logging to stderr
- `SERIAL_MCP_HTTP_BIND=0.0.0.0:8000` — override HTTP bind address
- `SERIAL_MCP_ALLOWLIST="/dev/ttyACM*,/dev/ttyUSB*"` — restrict allowed ports

### Configure Claude Desktop

```json
{
  "mcpServers": {
    "serial": {
      "command": "/path/to/serial-mcp-server/target/release/serial-mcp-server",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM0"
      }
    }
  }
}
```

## Supported Hardware

Works with any UART or USB-serial device:

- **Boards:** STM32, Arduino (Uno/Nano/Leonardo), ESP32, ESP8266
- **Chips:** CH340/CP2102/FT232 and native USB-CDC
- **Platforms:** Windows (`COMx`), Linux (`/dev/tty*`), macOS (`/dev/tty.*`)

On Linux, add your user to the `dialout` group for `/dev/tty*` access.

## Example Agent Flow

```
1. list_ports → ["/dev/ttyUSB0", "/dev/ttyACM0"]
2. open(port="/dev/ttyACM0", baud_rate=115200) → { connection_id: "9f..." }
3. set_dtr_rts(id, dtr=false, rts=false)  # Arduino soft-reset
   set_dtr_rts(id, dtr=true,  rts=true)
4. wait_for(id, pattern="OK>", timeout_ms=3000) → { matched: true, ... }
5. write(id, data="status\r\n")
   wait_for(id, pattern="OK>", timeout_ms=1000)
6. close(id)
```

For passive monitoring, use `subscribe(id)` to receive all RX bytes as
MCP `notifications/message` events.

## Commands

```bash
cargo test                    # Full test suite (~62 tests)
cargo clippy --all-targets -- -D warnings   # Lint (zero warnings)
cargo fmt --all -- --check    # Format check

# Hardware tests (requires serial device with TX-RX loopback)
SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored
```

## Documentation

- [CHANGELOG.md](CHANGELOG.md) — Version history
- [AGENTS.md](AGENTS.md) — Coding guidelines for contributors
- [REVIEW.md](REVIEW.md) — Code walkthrough and design notes
- [examples/STM32_demo/](examples/STM32_demo/) — Demo firmware

## License

MIT. See [LICENSE](LICENSE).
