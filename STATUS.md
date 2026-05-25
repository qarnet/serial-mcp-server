# Serial MCP Server — v0.2.2 Complete

**Status:** All planned features implemented and tested. MCP 2025-11-25 compliance audit fixed.

## What's New in v0.2.2

- **MCP Compliance ~85%** — SPECIFICATION_COMPLIANCE.md fixed (false negatives corrected)
- **Functional pagination** — cursor-based pagination with nextCursor for list operations
- **Resource metadata** — size, priority, and audience annotations on all resources/templates
- **Tool outputSchema verified** — All 11 tools auto-generate output schemas via rmcp macro
- **Dead code removed** — Unused processor/tool_router/prompt_router fields + task_handler macro
- **70+ active tests** — 0 failures, clippy clean

## What's New in v0.2.1

- **MCP 2025-11-25 compliant** — Protocol version updated
- **Resource change notifications** — Push updates on open/close
- **Port allowlist** — `SERIAL_MCP_ALLOWLIST` with glob patterns
- **CDC-ACM fixes** — Hardware tested on `/dev/ttyACM0`
- **STDIO transport tests** — Verified with real child process

## Quick Commands

```bash
# Run all tests (~70 tests)
cargo test

# Run hardware tests (requires device at SERIAL_MCP_TEST_PORT)
SERIAL_MCP_TEST_PORT=/dev/ttyACM0 cargo test --test hardware_loopback -- --ignored --test-threads=1

# Run with allowlist
SERIAL_MCP_ALLOWLIST="/dev/ttyACM0" ./serial-mcp-server

# CI gate
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Documentation

- `README.md` — Updated with all new features
- `CHANGELOG.md` — v0.2.2 and v0.2.1 entries
- `AGENTS.md` — Coding guidelines for contributors
- `REVIEW.md` — Code walkthrough and design notes
- `PLAN.md` — Implementation plan (all phases complete)
- `SPECIFICATION_COMPLIANCE.md` — MCP spec compliance report (~85%)

## See Also

- [README.md](README.md) for full documentation
- [CHANGELOG.md](CHANGELOG.md) for version history
- [REVIEW.md](REVIEW.md) for code walkthrough
- [SPECIFICATION_COMPLIANCE.md](SPECIFICATION_COMPLIANCE.md) for MCP compliance details
