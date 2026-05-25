# Plan: Fix Non-Standard JSON Schema Format Warnings

## Problem

`schemars` 1.0.4 generates non-standard `format` values for unsigned integer types:
- `usize` → `"format": "uint"`
- `u32` → `"format": "uint32"`  
- `u64` → `"format": "uint64"`

These are not part of JSON Schema Draft 2020-12. Opencode logs warnings but works correctly.

## Solution: Per-Field `schema_with` Overrides (Option 1)

Use `#[schemars(schema_with = "...")]` on every affected field to emit `"type": "integer", "minimum": 0` without the non-standard `format` keyword.

## Implementation Steps

### 1. Create Schema Helper Functions

Add to `src/tools/types.rs` (or a new `src/schema_helpers.rs`):

```rust
use schemars::{SchemaGenerator, Schema, json_schema};

/// Schema for u32/usize/u64 without non-standard `format` keyword.
/// Emits {"type": "integer", "minimum": 0}.
pub fn uint_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "type": "integer",
        "minimum": 0
    })
}

/// Schema for Option<usize/u32/u64> without non-standard `format`.
/// Emits anyOf [null, {"type": "integer", "minimum": 0}].
pub fn option_uint_schema(_gen: &mut SchemaGenerator) -> Schema {
    json_schema!({
        "anyOf": [
            {"type": "null"},
            {"type": "integer", "minimum": 0}
        ]
    })
}
```

### 2. Annotate All Affected Fields

#### `src/tools/types.rs`

**Argument structs:**
- `ReadArgs.timeout_ms` — `Option<u64>` → `#[schemars(schema_with = "crate::schema_helpers::option_uint_schema")]`
- `ReadArgs.max_bytes` — `usize` → `#[schemars(schema_with = "crate::schema_helpers::uint_schema")]`
- `SendBreakArgs.duration_ms` — `u64` → `#[schemars(schema_with = "crate::schema_helpers::uint_schema")]`
- `SubscribeArgs.max_chunk_bytes` — `usize` → `#[schemars(schema_with = "crate::schema_helpers::uint_schema")]`
- `SubscribeArgs.poll_interval_ms` — `u64` → `#[schemars(schema_with = "crate::schema_helpers::uint_schema")]`
- `WaitForArgs.timeout_ms` — `u64` → `#[schemars(schema_with = "crate::schema_helpers::uint_schema")]`
- `WaitForArgs.max_bytes` — `usize` → `#[schemars(schema_with = "crate::schema_helpers::uint_schema")]`

**Response structs:**
- `ListPortsResult.count` — `usize` → `uint_schema`
- `OpenResult.baud_rate` — `u32` → `uint_schema`
- `WriteResult.bytes_written` — `usize` → `uint_schema`
- `ReadResult.bytes_read` — `usize` → `uint_schema`
- `ReadResult.timeout_ms` — `u64` → `uint_schema`
- `SendBreakResult.duration_ms` — `u64` → `uint_schema`
- `SubscribeResult.max_chunk_bytes` — `usize` → `uint_schema`
- `SubscribeResult.poll_interval_ms` — `u64` → `uint_schema`
- `WaitForResult.bytes_read` — `usize` → `uint_schema`
- `WaitForResult.match_index` — `Option<usize>` → `option_uint_schema`
- `WaitForResult.timeout_ms` — `u64` → `uint_schema`

#### `src/resources/types.rs`

- `ConnectionsResource.count` — `usize` → `uint_schema`

#### `src/prompts/types.rs`

- `DiagnosePortArgs.baud_rate` — `Option<u32>` → `option_uint_schema`

Note: `serial.rs` (`PortInfo`, `ConnectionSummary`, `FlushTarget`) does not appear in the err.log warnings. Only add annotations if tests reveal they also need it.

### 3. Verify `serde` Behavior Is Unchanged

The `#[schemars(...)]` attribute only affects schema generation, not serialization/deserialization. All `#[serde(...)]` attributes remain untouched.

### 4. Run Build & Tests

```bash
cargo build --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Test Strategy

### Test A: No Non-Standard Formats in Tool Schemas

Add to `src/tools/mod.rs` (inside existing `verify_all_tool_schemas` test or as a new test):

```rust
#[test]
fn tool_schemas_have_no_nonstandard_uint_formats() {
    let tools = vec![
        SerialHandler::list_ports_tool_attr(),
        SerialHandler::open_tool_attr(),
        SerialHandler::close_tool_attr(),
        SerialHandler::write_tool_attr(),
        SerialHandler::read_tool_attr(),
        SerialHandler::flush_tool_attr(),
        SerialHandler::set_dtr_rts_tool_attr(),
        SerialHandler::send_break_tool_attr(),
        SerialHandler::subscribe_tool_attr(),
        SerialHandler::unsubscribe_tool_attr(),
        SerialHandler::wait_for_tool_attr(),
    ];
    
    for tool in tools {
        let schema_str = serde_json::to_string(&tool).unwrap();
        assert!(
            !schema_str.contains("\"format\":\"uint\""),
            "schema for {} contains non-standard 'uint' format",
            tool.name
        );
        assert!(
            !schema_str.contains("\"format\":\"uint32\""),
            "schema for {} contains non-standard 'uint32' format",
            tool.name
        );
        assert!(
            !schema_str.contains("\"format\":\"uint64\""),
            "schema for {} contains non-standard 'uint64' format",
            tool.name
        );
    }
}
```

This catches the exact strings appearing in `err.log`.

### Test B: Unsigned Integer Fields Still Have `minimum: 0`

Verify the override didn't accidentally drop the unsigned constraint:

```rust
#[test]
fn open_args_schema_has_minimum_zero_for_baud_rate() {
    let schema = schemars::schema_for!(OpenArgs);
    let props = schema.schema_object.properties.as_ref().unwrap();
    let baud = props.get("baud_rate").unwrap();
    let json = serde_json::to_value(baud).unwrap();
    assert_eq!(json.get("minimum"), Some(&serde_json::json!(0)));
}
```

### Test C: Resource Schemas Are Clean

```rust
#[test]
fn connections_resource_schema_has_no_uint_format() {
    let schema = schemars::schema_for!(ConnectionsResource);
    let json = serde_json::to_string(&schema).unwrap();
    assert!(!json.contains("\"format\":\"uint\""));
}
```

## Edge Cases

1. **`Option<T>` fields**: `#[schemars(schema_with = "...")]` on an `Option` field generates the custom schema directly (not wrapped in Option). Must provide `anyOf [null, integer]` manually.
2. **`usize` on 32-bit vs 64-bit**: The `"uint"` format is always non-standard regardless of pointer width. The override is portable.
3. **Nested structs**: If any struct fields contain other structs with unsigned integers (e.g., `Vec<PortInfo>`), only the top-level fields in `err.log` need fixing. `PortInfo` has no unsigned fields.
4. **Derive macro interaction**: `#[schemars(schema_with)]` overrides the derive-generated schema for that field only. Other fields in the same struct continue using derive defaults.

## Files to Touch

1. `src/tools/types.rs` — add helpers + annotate ~17 fields
2. `src/resources/types.rs` — annotate 1 field
3. `src/prompts/types.rs` — annotate 1 field
4. `src/tools/mod.rs` — add Test A (schema format check)
5. `Cargo.toml` — no changes needed (schemars already a dependency)

## Rollback

If this causes issues: revert the `schema_with` annotations and the helper module. `#[schemars]` attributes are compile-time only; runtime behavior is unchanged.
