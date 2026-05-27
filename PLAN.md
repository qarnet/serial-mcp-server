# Distribution & Release Plan

## Goal

Make `serial-mcp-server` trivially installable for any agent user —
pre-built binaries, copy-paste config snippets, discoverable via GitHub
Releases. No Rust toolchain or `libudev-dev` required.

## Current state

| Asset | Status |
|---|---|
| GitHub Actions CI | Build + test + clippy on push/PR |
| GitHub Releases | None (tags exist but no artifacts) |
| `Cargo.toml` version | `0.2.2` — stale (tags reach v0.2.6) |
| `CHANGELOG.md` | Last entry is v0.2.2 — missing v0.2.3–v0.2.6 |
| Nix flake | Dev shell + `crane` package + aarch64 cross-compile |
| README | Good tool surface docs, Claude Desktop config example |
| Agent config snippets | None in repo (only `.claude/settings.local.json`) |
| Pre-built binaries | None — users must `cargo build --release` |

## Plan

### Phase 1: Housekeeping (~10 min)

**1.1 Bump `Cargo.toml` version** — `0.2.2` → `0.2.6`

```toml
[package]
version = "0.2.6"
```

**1.2 Fill in CHANGELOG** — Add entries for v0.2.3 through v0.2.6:

| Version | Summary |
|---|---|
| v0.2.3 | Subscribe `timeout_ms` blocking mode (fire-and-forget preserved) |
| v0.2.4 | Schema fix: remove `skip_serializing_if` on optional fields |
| v0.2.5 | Proptest property tests (54), cargo-fuzz targets (3), lifecycle tests |
| v0.2.6 | Protocol emulator integration tests (ESP32 weather-station + binary payload) |

### Phase 2: GitHub Release CD (~30 min)

Add `.github/workflows/release.yml` triggered on `v*` tag push.

**2.1 x86_64-linux binary release (native)**

```yaml
name: Release
on:
  push:
    tags: ["v*"]
jobs:
  release-x86_64:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - run: sudo apt-get install -y libudev-dev pkg-config
      - run: cargo build --release --locked
      - run: strip target/release/serial-mcp-server target/release/serial-mcp-server-http
      - uses: softprops/action-gh-release@v2
        with:
          generate_release_notes: true
          files: |
            target/release/serial-mcp-server
            target/release/serial-mcp-server-http
```

Produces two statically-linkable binaries. Users do:

```bash
curl -L https://github.com/qarnet/serial-mcp-server/releases/latest/download/serial-mcp-server -o serial-mcp-server
chmod +x serial-mcp-server
```

**2.2 aarch64-linux cross-compile via Nix (optional, Phase 2b)**

The flake already cross-compiles to `aarch64-unknown-linux-gnu`. A
second job in the release workflow using `nix build .#serial-mcp-server-aarch64`
produces Raspberry Pi / ARM64 binaries. Add after x86_64 is working.

### Phase 3: README overhaul (~20 min)

Replace the "Quick Start" section with a proper install guide:

```
## Install

### Option A: Pre-built binary (recommended)

curl -L https://github.com/qarnet/serial-mcp-server/releases/latest/download/serial-mcp-server -o serial-mcp-server
chmod +x serial-mcp-server
sudo mv serial-mcp-server /usr/local/bin/

### Option B: Nix flake

nix profile install github:qarnet/serial-mcp-server

### Option C: cargo install

cargo install serial-mcp-server
```

**Add agent config section:**

```json
// opencode.json — add to your mcpServers block
{
  "mcpServers": {
    "serial": {
      "type": "stdio",
      "command": "/usr/local/bin/serial-mcp-server",
      "env": {
        "RUST_LOG": "info",
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM*,/dev/ttyUSB*"
      }
    }
  }
}
```

```json
// claude_desktop_config.json — same pattern
{
  "mcpServers": {
    "serial": {
      "command": "/usr/local/bin/serial-mcp-server",
      "args": [],
      "env": {
        "RUST_LOG": "info",
        "SERIAL_MCP_ALLOWLIST": "/dev/ttyACM0"
      }
    }
  }
}
```

**Add badges:** GitHub Release (latest), Nix flake, license.

### Phase 4: Optional polish

**4.1 `cargo install` support** — Already works (`Cargo.toml` has `[[bin]]`
entries). The `cargo install serial-mcp-server` path gives users two
binaries. Document the `libudev-dev` requirement on Linux.

**4.2 Homebrew tap** — For macOS users. Create a `homebrew-serial-mcp`
repo with a formula pointing at the GitHub Release binaries. Low
effort, high reach.

**4.3 `.opencode.json` example** — Ship an example config in
`examples/configs/opencode.json` so users can see exactly what to drop
in.

## Implementation order

1. **Housekeeping** — bump version, fill changelog. Commit + tag v0.2.7.
2. **Release workflow** — add `.github/workflows/release.yml`. Push.
3. **README overhaul** — install section, agent config snippets, badges.
4. **Optional: Nix aarch64 in release** — second release job.
5. **Optional: example configs directory** — `examples/configs/`.

## What NOT to do

- Don't add `version` to the release binaries themselves (no embedded version
  string needed — the MCP server doesn't expose `--version`).
- Don't build macOS/Windows binaries yet — `serialport` requires the native
  platform SDK which complicates CI. Ship Linux first, add others when
  users ask.
- Don't add a `.deb` / `.rpm` — a raw binary is simpler and the tool has no
  config files or systemd units.
- Don't remove the `cargo build` instructions from README — keep them as
  a fallback for developers.
