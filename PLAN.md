# Serial MCP Server — Implementation Plan

**Date:** 2026-05-25  
**Status:** All Phases Complete (v0.2.2)

---

## Architecture Decisions

### Streaming Strategy: Hybrid Approach

**Decision:** Use both channels — each for its semantic purpose:

- **`notifications/message`** (logging channel): **RX data streaming only**
  - Immediate delivery of serial bytes from `subscribe` tool
  - No extra round-trip required
  - Works with all MCP clients (backward compatible)
  - Most efficient for high-frequency streaming

- **`resources/updated`**: **Connection lifecycle events only**
  - Fired when connections open (`open` tool) or close (`close` tool)
  - Signals to clients that `serial://connections` resource has changed
  - Enables agents to refresh their view of open ports without polling

- **`resources/list_changed` capability**: Added to `get_info()`
  - Declares that the server supports push notifications for resource list changes
  - Clients subscribe once and get notified of changes

**Why this approach:**
- Data streaming via logging is immediate (best UX for real-time serial monitoring)
- Resource notifications are correct for lifecycle events (spec-compliant)
- No confusion about which channel to use for what purpose
- Backward compatible with clients that only support logging notifications

---

## Completed Implementation

### ✅ Phase 1: Critical Fixes (v0.2.1)

#### 1.1 Hardware Loopback Tests for CDC-ACM
**Status:** COMPLETE - Both hardware tests pass on `/dev/ttyACM0`

**Fix:** Changed `POLL_MS` from 5ms to 50ms in `SerialConnection::read()` to allow CDC-ACM USB packets to coalesce before returning.

#### 1.2 Update Protocol Version to 2025-11-25
**Status:** COMPLETE

#### 1.3 Add Resource Change Notifications
**Status:** COMPLETE - `open` and `close` tools fire `notify_resource_list_changed()`

#### 1.4 RX Streaming via notifications/message
**Status:** COMPLETE - Confirmed working as designed

### ✅ Phase 2: Testing (v0.2.1)

#### 2.1 Hardware Loopback Tests
**Status:** COMPLETE - Verified on `/dev/ttyACM0`

#### 2.2 STDIO Transport Tests
**Status:** COMPLETE

#### 2.3 Resource Notification Tests
**Status:** COMPLETE

### ✅ Phase 3: Runtime Configuration (v0.2.1)

#### 3.1 Port Allowlist Configuration
**Status:** COMPLETE - `SERIAL_MCP_ALLOWLIST` with glob patterns

### ✅ Phase 4: MCP Compliance Audit Fixes (v0.2.2)

#### 4.1 Pagination
**Status:** COMPLETE - Functional cursor-based pagination for `list_resources` and `list_resource_templates`
- `PaginatedRequestParams` cursor parameter interpreted as base64-encoded offset
- Page size: 100 items
- `nextCursor` properly populated

#### 4.2 Tool outputSchema
**Status:** COMPLETE - All 11 tools have auto-generated output schemas via rmcp macro
- Verified by `tools::tests::verify_all_tool_schemas` test

#### 4.3 Resource Metadata
**Status:** COMPLETE - Added `size` field to resources and templates
- `serial://ports`: size = port count
- `serial://connections`: size = connection count

#### 4.4 SPECIFICATION_COMPLIANCE.md
**Status:** COMPLETE - Fixed false negatives:
- `title`: marked ✅ (present on all tools)
- `annotations`: marked ✅ (present on relevant tools)
- `progressToken`: marked ✅ (wired for read/wait_for/send_break)
- `CancellationToken`: marked ✅ (cooperative cancellation working)

#### 4.5 Dead Code Removal
**Status:** COMPLETE - Removed unused fields and macros:
- Removed `processor`, `tool_router`, `prompt_router` from `SerialHandler`
- Removed unused `#[task_handler]` macro

---

## Test Summary (v0.2.2)

| Layer | File | Count | Status |
|---|---|---|---|
| 1 — Unit | `src/*.rs` | 37 | ✅ All pass |
| 2 — HTTP Integration | `tests/http_integration.rs` | 17 | ✅ All pass |
| 3 — Resource Subscriptions | `tests/resource_subscriptions.rs` | 2 | ✅ All pass |
| 4 — Allowlist | `tests/allowlist.rs` | 3 | ✅ All pass |
| 5 — PTY | `tests/serial_pty.rs` | 6 | ✅ All pass |
| 6 — Hardware Loopback | `tests/hardware_loopback.rs` | 2 | ✅ All pass (on `/dev/ttyACM0`) |
| 7 — STDIO Integration | `tests/stdio_integration.rs` | 3+1 | ✅ 3 pass, 1 ignored |

**Total:** 70 tests active, 2 ignored, 0 failures

**CI Gate:** `cargo clippy --all-targets -- -D warnings` ✅ clean

---

## Remaining Work (Phase 5 — Future)

### 5.1 Connection Statistics Resource
**Priority:** Low
**Status:** NOT STARTED

**Resource:** `serial://connections/{id}/stats`
- Track bytes sent/received per connection
- Track read timeouts, write errors
- Add resource template for per-connection stats

### 5.2 Cross-process Port Locking (Advisory)
**Priority:** Low
**Status:** NOT STARTED

**Problem:** Two separate server processes may both open the same physical port.

**Suggestion:** Add an *advisory lockfile per port* (e.g. `flock` on a hashed filename under `/var/lock/serial-mcp-server/`) held for the lifetime of an open connection.

---

## Files Modified (v0.2.2)

### Source code
- `src/server.rs` - Pagination, resource metadata, dead code removal
- `src/serial.rs` - Added `ConnectionManager::count()` method
- `src/tools/mod.rs` - Added outputSchema verification test

### Documentation
- `SPECIFICATION_COMPLIANCE.md` - Fixed false negatives, updated scores
- `CHANGELOG.md` - Added v0.2.2 entry
- `AGENTS.md` - Updated with unit test examples

### Tests
- `tests/http_integration.rs` - Added pagination tests

---

## Success Criteria Met

- [x] All 70 tests pass (37 unit + 17 HTTP + 2 resource_subscriptions + 3 allowlist + 6 PTY + 3 stdio)
- [x] Hardware loopback tests pass on `/dev/ttyACM0`
- [x] Pagination functional with cursor/nextCursor
- [x] All tools have outputSchema
- [x] Resource metadata (size, priority, audience) added
- [x] SPECIFICATION_COMPLIANCE.md accurate (85% score)
- [x] Dead code removed (processor, tool_router, prompt_router, task_handler)
- [x] CI passes: `cargo test`, `cargo clippy -D warnings`
