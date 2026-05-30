# Serial MCP Server

[![GitHub Release](https://img.shields.io/github/v/release/qarnet/serial-mcp-server)](https://github.com/qarnet/serial-mcp-server/releases)
[![crates.io](https://img.shields.io/crates/v/serial-mcp-server)](https://crates.io/crates/serial-mcp-server)
[![Rust](https://img.shields.io/badge/rust-1.70+-orange.svg)](https://rust-lang.org)
[![RMCP](https://img.shields.io/badge/RMCP-1.7-blue.svg)](https://github.com/modelcontextprotocol/rust-sdk)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

An MCP (Model Context Protocol) server that lets AI assistants drive serial
ports: open, read, write, wait for prompts, stream RX bytes, toggle DTR/RTS,
send BREAK.

**MCP 2025-11-25 compliant** · resource change notifications · port allowlist · stdio + HTTP transports

## What It Does

Exposes serial ports as MCP tools so agents like Claude can interact with
embedded devices, Arduino boards, STM32 microcontrollers, and any UART/USB-serial
hardware — all through natural language.

**11 tools** — list_ports, open, close, read, write, flush, set_dtr_rts, send_break, wait_for, subscribe, unsubscribe  
**3 resources** — `serial://ports`, `serial://connections`, `serial://connections/{id}`  
**2 prompt templates** — `diagnose_port`, `interactive_terminal`  
**Live RX streaming** — background task pushes bytes as MCP notifications  
**Task cancellation** — cancel long-running reads/waits via `tasks/cancel`  

## Install

### Linux

```bash
# Pre-built binary (x86_64)
VERSION=$(curl -s https://api.github.com/repos/qarnet/serial-mcp-server/releases/latest | grep -oP '"tag_name": "\K[^"]+')
curl -L "https://github.com/qarnet/serial-mcp-server/releases/download/${VERSION}/serial-mcp-server-${VERSION#v}-x86_64-linux" \
  -o serial-mcp-server
chmod +x serial-mcp-server
sudo mv serial-mcp-server /usr/local/bin/
```

```bash
# Via cargo
cargo install serial-mcp-server
# or, until crates.io publish:
cargo install --git https://github.com/qarnet/serial-mcp-server serial-mcp-server
```

```bash
# Via Nix
nix profile install github:qarnet/serial-mcp-server
```

> **Serial port access:** add your user to the `dialout` group: `sudo usermod -aG dialout $USER` (re-login required).

### macOS

```bash
# Pre-built binary
VERSION=$(curl -s https://api.github.com/repos/qarnet/serial-mcp-server/releases/latest | grep -oP '"tag_name": "\K[^"]+')
# Apple Silicon (M1/M2/M3/M4):
ARCH=aarch64-macos
# Intel Mac:
# ARCH=x86_64-macos
curl -L "https://github.com/qarnet/serial-mcp-server/releases/download/${VERSION}/serial-mcp-server-${VERSION#v}-${ARCH}" \
  -o serial-mcp-server
chmod +x serial-mcp-server
sudo mv serial-mcp-server /usr/local/bin/
```

```bash
# Via cargo (no extra dependencies needed)
cargo install serial-mcp-server
```

> **Serial port access:** macOS may prompt for permission when a serial device is first opened. Grant it in System Settings → Privacy & Security → Files and Folders (or via the dialog that appears).

### Windows

**Pre-built binary:** download `serial-mcp-server-{VERSION}-x86_64-windows.exe` from the [latest release](https://github.com/qarnet/serial-mcp-server/releases/latest), rename it to `serial-mcp-server.exe`, and place it somewhere on your `PATH` (e.g. `C:\tools\`).

```powershell
# Via cargo (no extra dependencies needed — install Rust from https://rustup.rs)
cargo install serial-mcp-server
```

> **Serial port access:** COM ports are usually accessible without extra configuration. If a port is in use by another program, close it first.

## Wire Up Your Agent

The binary runs as a stdio MCP server by default. Point your agent at it with the path to the installed binary and set `SERIAL_MCP_ALLOWLIST` to restrict which ports can be opened.

### Claude Code CLI

Add to `.claude/settings.json` in your project, or `~/.claude/settings.json` globally:

```json
{
  "mcpServers": {
    "serial": {
      "command": "/usr/local/bin/serial-mcp-server",
      "env": {
        "RUST_LOG": "warn",
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*,/dev/ttyUSB*"
      }
    }
  }
}
```

<details>
<summary>Windows variant</summary>

```json
{
  "mcpServers": {
    "serial": {
      "command": "C:\\Users\\<user>\\.cargo\\bin\\serial-mcp-server.exe",
      "env": {
        "RUST_LOG": "warn",
        "SERIAL_MCP_ALLOWLIST": "COM3,COM4"
      }
    }
  }
}
```

</details>

### Claude Desktop

Config file locations:
- **Linux:** `~/.config/claude-desktop/claude_desktop_config.json`
- **macOS:** `~/Library/Application Support/Claude/claude_desktop_config.json`
- **Windows:** `%APPDATA%\Claude\claude_desktop_config.json`

```json
{
  "mcpServers": {
    "serial": {
      "command": "/usr/local/bin/serial-mcp-server",
      "args": [],
      "env": {
        "RUST_LOG": "warn",
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM0"
      }
    }
  }
}
```

<details>
<summary>macOS / Windows variants</summary>

macOS:
```json
{
  "mcpServers": {
    "serial": {
      "command": "/Users/<user>/.cargo/bin/serial-mcp-server",
      "env": {
        "SERIAL_MCP_ALLOWLIST": "/dev/tty.usbmodem*,/dev/tty.usbserial-*"
      }
    }
  }
}
```

Windows:
```json
{
  "mcpServers": {
    "serial": {
      "command": "C:\\Users\\<user>\\.cargo\\bin\\serial-mcp-server.exe",
      "env": {
        "SERIAL_MCP_ALLOWLIST": "COM3,COM4"
      }
    }
  }
}
```

</details>

### Cursor

Add to `.cursor/mcp.json` in your project root, or `~/.cursor/mcp.json` globally:

```json
{
  "mcpServers": {
    "serial": {
      "command": "/usr/local/bin/serial-mcp-server",
      "env": {
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*,/dev/ttyUSB*"
      }
    }
  }
}
```

### VS Code (Copilot)

Add to `.vscode/mcp.json` in your workspace:

```json
{
  "servers": {
    "serial": {
      "type": "stdio",
      "command": "/usr/local/bin/serial-mcp-server",
      "env": {
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*,/dev/ttyUSB*"
      }
    }
  }
}
```

### Zed

Add to `~/.config/zed/settings.json` under `"context_servers"`:

```json
{
  "context_servers": {
    "serial-mcp-server": {
      "command": {
        "path": "/usr/local/bin/serial-mcp-server",
        "args": []
      },
      "settings": {}
    }
  }
}
```

### opencode

Add to `opencode.json` or `opencode.jsonc` in your project or `~/.config/opencode/opencode.json`:

```json
{
  "mcpServers": {
    "serial": {
      "type": "stdio",
      "command": "/usr/local/bin/serial-mcp-server",
      "env": {
        "RUST_LOG": "warn",
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*,/dev/ttyUSB*"
      }
    }
  }
}
```

### HTTP transport (remote / headless)

Use `--transport=http` to expose the server over HTTP instead of stdio. Useful for running the server on a headless machine (e.g. a Pi with USB dongles) while agents connect remotely.

```json
{
  "mcpServers": {
    "serial": {
      "type": "streamable-http",
      "url": "http://127.0.0.1:8000/mcp"
    }
  }
}
```

Start the server on the target machine:

```bash
serial-mcp-server --transport=http
# or via environment variable:
SERIAL_MCP_TRANSPORT=http serial-mcp-server
```

Override the bind address with `SERIAL_MCP_HTTP_BIND` (default `127.0.0.1:8000`).

### Dev one-liner (no install needed)

If you have the repo cloned, agents can build and run the server on demand:

```json
{
  "command": "cargo",
  "args": [
    "run", "--quiet", "--manifest-path", "/path/to/serial-mcp-server/Cargo.toml",
    "--bin", "serial-mcp-server", "--"
  ],
  "env": {
    "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*"
  }
}
```

## Platform Port Names

| Platform | Example ports | Notes |
|---|---|---|
| Linux | `/dev/ttyACM0`, `/dev/ttyUSB0` | Add user to `dialout` group |
| macOS | `/dev/tty.usbmodem1101`, `/dev/tty.usbserial-*` | Grant serial permission on first use |
| Windows | `COM3`, `COM4` | No extra setup needed |

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `RUST_LOG` | `info` | Logging level (`error`, `warn`, `info`, `debug`, `trace`) |
| `SERIAL_MCP_HTTP_BIND` | `127.0.0.1:8000` | HTTP transport bind address |
| `SERIAL_MCP_ALLOWLIST` | *(empty = allow all)* | Comma-separated glob patterns for allowed ports |
| `SERIAL_MCP_TRANSPORT` | `stdio` | Transport to use (`stdio` or `http`) |

## Transports

| Flag / env | Transport | When to use |
|---|---|---|
| *(default)* | stdio | Desktop agents (Claude Code, Claude Desktop, Cursor, VS Code, Zed) |
| `--transport=http` | streamable HTTP | Remote access, headless servers, CI pipelines |

## Supported Hardware

Works with any UART or USB-serial device:

- **Boards:** STM32, Arduino (Uno/Nano/Leonardo), ESP32, ESP8266
- **Chips:** CH340/CP2102/FT232 and native USB-CDC
- **Platforms:** Windows (`COMx`), Linux (`/dev/tty*`), macOS (`/dev/tty.*`)

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

## Publishing to crates.io

Add a `CARGO_REGISTRY_TOKEN` secret to your GitHub repository settings. The release workflow publishes automatically on each version tag.

To publish manually:
```bash
cargo publish
```

## Commands

```bash
cargo test                    # Full test suite (~140 tests)
cargo clippy --all-targets -- -D warnings   # Lint (zero warnings)
cargo fmt --all -- --check    # Format check

# Hardware tests (requires serial device with TX-RX loopback)
SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored
```

## Documentation

- [CHANGELOG.md](CHANGELOG.md) — Version history
- [AGENTS.md](AGENTS.md) — Coding guidelines for contributors
- [REVIEW.md](REVIEW.md) — Code walkthrough and design notes

## License

MIT. See [LICENSE](LICENSE).
