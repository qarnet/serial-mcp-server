# Serial MCP Server

[![GitHub Release](https://img.shields.io/github/v/release/qarnet/serial-mcp-server)](https://github.com/qarnet/serial-mcp-server/releases)
[![crates.io](https://img.shields.io/crates/v/serial-mcp-server)](https://crates.io/crates/serial-mcp-server)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

An MCP server that lets AI assistants drive serial ports: open, read, write,
wait for prompts, stream RX bytes, toggle DTR/RTS, send BREAK.

**MCP 2025-11-25 compliant** · resource change notifications · port allowlist · stdio + HTTP transports

## What It Does

Exposes serial ports as MCP tools so agents like Claude can interact with
embedded devices, Arduino boards, STM32 microcontrollers, and any UART/USB-serial
hardware — all through natural language.

**11 tools** — list_ports, open, close, read, write, flush, set_dtr_rts, send_break, wait_for, subscribe, unsubscribe  
**3 resources** — `serial://ports`, `serial://connections`, `serial://connections/{id}`  
**2 prompt templates** — `diagnose_port`, `interactive_terminal`  

## Install

### Linux

```bash
VERSION=$(curl -s https://api.github.com/repos/qarnet/serial-mcp-server/releases/latest | grep -oP '"tag_name": "\K[^"]+')
curl -L "https://github.com/qarnet/serial-mcp-server/releases/download/${VERSION}/serial-mcp-server-${VERSION#v}-x86_64-linux" \
  -o serial-mcp-server && chmod +x serial-mcp-server && sudo mv serial-mcp-server /usr/local/bin/
```

Add user to `dialout` group for port access: `sudo usermod -aG dialout $USER`

### macOS

```bash
VERSION=$(curl -s https://api.github.com/repos/qarnet/serial-mcp-server/releases/latest | grep -oP '"tag_name": "\K[^"]+')
ARCH=aarch64-macos   # Intel: x86_64-macos
curl -L "https://github.com/qarnet/serial-mcp-server/releases/download/${VERSION}/serial-mcp-server-${VERSION#v}-${ARCH}" \
  -o serial-mcp-server && chmod +x serial-mcp-server && sudo mv serial-mcp-server /usr/local/bin/
```

### Windows

Download `serial-mcp-server-{VERSION}-x86_64-windows.exe` from the [latest release](https://github.com/qarnet/serial-mcp-server/releases/latest) and place it on your `PATH`.

### Via cargo (all platforms)

```bash
cargo install serial-mcp-server
```

### Via Nix

```bash
nix profile install github:qarnet/serial-mcp-server
```

## Wire Up Your Agent

→ **[Agent configuration guide](docs/agent-config.md)** — Claude Code CLI, Claude Desktop, Cursor, VS Code, Zed, opencode, HTTP transport

Quick example (Claude Code CLI, Linux/macOS):

```json
{
  "mcpServers": {
    "serial": {
      "command": "/usr/local/bin/serial-mcp-server",
      "env": { "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*,/dev/ttyUSB*" }
    }
  }
}
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `info` | Log level (`error` `warn` `info` `debug`) |
| `SERIAL_MCP_ALLOWLIST` | *(empty = allow all)* | Comma-separated glob patterns for allowed ports |
| `SERIAL_MCP_HTTP_BIND` | `127.0.0.1:8000` | HTTP transport bind address |
| `SERIAL_MCP_TRANSPORT` | `stdio` | Transport: `stdio` or `http` |

## Transports

| Mode | How to activate | Use case |
|---|---|---|
| stdio | default | Desktop agents |
| HTTP | `--transport=http` or `SERIAL_MCP_TRANSPORT=http` | Remote / headless |

## Supported Hardware

- **Boards:** STM32, Arduino (Uno/Nano/Leonardo), ESP32, ESP8266
- **Chips:** CH340, CP2102, FT232, native USB-CDC
- **Platforms:** Windows (`COMx`), Linux (`/dev/tty*`), macOS (`/dev/tty.*`)

## Example Agent Flow

```
1. list_ports → ["/dev/ttyUSB0", "/dev/ttyACM0"]
2. open(port="/dev/ttyACM0", baud_rate=115200) → { connection_id: "9f..." }
3. set_dtr_rts(id, dtr=false, rts=false)  # Arduino reset
   set_dtr_rts(id, dtr=true,  rts=true)
4. wait_for(id, pattern="OK>", timeout_ms=3000)
5. write(id, data="status\r\n")
6. close(id)
```

## Development

```bash
cargo test                                          # ~140 tests
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check

# Hardware tests (requires TX-RX loopback device)
SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored
```

## Documentation

- [Agent Configuration](docs/agent-config.md)
- [CHANGELOG.md](CHANGELOG.md)
- [AGENTS.md](AGENTS.md) — contributor guidelines
- [REVIEW.md](REVIEW.md) — code walkthrough

## License

MIT. See [LICENSE](LICENSE).
