# Serial MCP Server — v0.2.1 Complete

**Status:** All planned features implemented and tested.

## What's New in v0.2.1

- **MCP 2025-11-25 compliant** — Protocol version updated
- **Resource change notifications** — Push updates on open/close
- **Port allowlist** — `SERIAL_MCP_ALLOWLIST` with glob patterns
- **CDC-ACM fixes** — Hardware tested on `/dev/ttyACM0`
- **STDIO transport tests** — Verified with real child process
- **62 active tests** — 0 failures, clippy clean

## Quick Commands

```bash
# Run all tests
cargo test

# Run hardware tests (requires device at SERIAL_MCP_TEST_PORT)
SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored --test-threads=1

# Run with allowlist
SERIAL_MCP_ALLOWLIST="/dev/ttyACM0" ./serial-mcp-server

# CI gate
cargo clippy --all-targets -- -D warnings
```

## Documentation

- `README.md` — Updated with all new features
- `CHANGELOG.md` — v0.2.1 entry added
- `REVIEW.md` — Updated for v0.2.1 codebase
- `PLAN.md` — Implementation plan (reference)

## See Also

- [README.md](README.md) for full documentation
- [CHANGELOG.md](CHANGELOG.md) for version history
- [REVIEW.md](REVIEW.md) for code walkthrough
