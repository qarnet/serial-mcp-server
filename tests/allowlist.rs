//! Test port allowlist functionality.

use rmcp::{
    model::CallToolRequestParams,
    transport::{child_process::TokioChildProcess, ConfigureCommandExt},
    ServiceExt,
};
use tokio::process::Command;

fn build_stdio_server() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let output = std::process::Command::new("cargo")
            .args(["build", "--bin", "serial-mcp-server"])
            .output()
            .expect("cargo build");
        if !output.status.success() {
            panic!(
                "cargo build --bin serial-mcp-server failed:\nstderr: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
    });
}

#[tokio::test]
async fn allowlist_blocks_unauthorized_port() {
    build_stdio_server();

    let cmd = Command::new(
        std::env::current_dir()
            .unwrap()
            .join("target/debug/serial-mcp-server"),
    )
    .configure(|cmd| {
        cmd.env("RUST_LOG", "off");
        // Only allow /dev/ttyACM1
        cmd.env("SERIAL_MCP_ALLOWLIST", "/dev/ttyACM1");
    });

    let transport = TokioChildProcess::new(cmd).expect("spawn stdio server");
    let client = ().serve(transport).await.expect("initialize client");

    // Try to open /dev/ttyACM0 (not in allowlist)
    let result = client
        .call_tool(
            CallToolRequestParams::new("open").with_arguments(
                serde_json::json!({
                    "port": "/dev/ttyACM0",
                    "baud_rate": 115200,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        )
        .await
        .unwrap();

    // Should fail with error
    assert_eq!(
        result.is_error,
        Some(true),
        "Expected open to be blocked by allowlist, got: {result:?}"
    );

    // Error message should mention allowlist
    let content = result
        .content
        .first()
        .and_then(|c| c.as_text())
        .map(|t| t.text.as_str())
        .unwrap_or("");
    assert!(
        content.contains("allowlist") || content.contains("not allowed"),
        "Error message should mention allowlist. Got: {content}"
    );

    client.cancel().await.ok();
}

#[tokio::test]
async fn allowlist_allows_authorized_port() {
    build_stdio_server();

    let cmd = Command::new(
        std::env::current_dir()
            .unwrap()
            .join("target/debug/serial-mcp-server"),
    )
    .configure(|cmd| {
        cmd.env("RUST_LOG", "off");
        // Allow the user's device
        cmd.env("SERIAL_MCP_ALLOWLIST", "/dev/ttyACM0");
    });

    let transport = TokioChildProcess::new(cmd).expect("spawn stdio server");
    let client = ().serve(transport).await.expect("initialize client");

    // Try to open /dev/ttyACM0 (in allowlist)
    let result = client
        .call_tool(
            CallToolRequestParams::new("open").with_arguments(
                serde_json::json!({
                    "port": "/dev/ttyACM0",
                    "baud_rate": 115200,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        )
        .await
        .unwrap();

    // Should succeed (or fail for OS reasons, but NOT for allowlist)
    if result.is_error == Some(true) {
        let content = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.as_str())
            .unwrap_or("");
        assert!(
            !content.contains("allowlist"),
            "Should not fail due to allowlist. Got: {content}"
        );
    }

    // Clean up if open succeeded
    if result.is_error != Some(true) {
        let structured = result.structured_content.expect("structured");
        let conn_id = structured["connection_id"].as_str().unwrap();
        client
            .call_tool(
                CallToolRequestParams::new("close").with_arguments(
                    serde_json::json!({
                        "connection_id": conn_id,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await
            .ok();
    }

    client.cancel().await.ok();
}

#[tokio::test]
async fn allowlist_glob_pattern_works() {
    build_stdio_server();

    let cmd = Command::new(
        std::env::current_dir()
            .unwrap()
            .join("target/debug/serial-mcp-server"),
    )
    .configure(|cmd| {
        cmd.env("RUST_LOG", "off");
        // Allow all /dev/ttyACM* and /dev/ttyUSB*
        cmd.env("SERIAL_MCP_ALLOWLIST", "/dev/ttyACM*,/dev/ttyUSB*");
    });

    let transport = TokioChildProcess::new(cmd).expect("spawn stdio server");
    let client = ().serve(transport).await.expect("initialize client");

    // Try to open /dev/ttyACM0 (matches glob)
    let result = client
        .call_tool(
            CallToolRequestParams::new("open").with_arguments(
                serde_json::json!({
                    "port": "/dev/ttyACM0",
                    "baud_rate": 115200,
                })
                .as_object()
                .unwrap()
                .clone(),
            ),
        )
        .await
        .unwrap();

    // Should succeed (or fail for OS reasons, but NOT for allowlist)
    if result.is_error == Some(true) {
        let content = result
            .content
            .first()
            .and_then(|c| c.as_text())
            .map(|t| t.text.as_str())
            .unwrap_or("");
        assert!(
            !content.contains("allowlist"),
            "Glob pattern should match. Got: {content}"
        );
    }

    // Clean up
    if result.is_error != Some(true) {
        let structured = result.structured_content.expect("structured");
        let conn_id = structured["connection_id"].as_str().unwrap();
        client
            .call_tool(
                CallToolRequestParams::new("close").with_arguments(
                    serde_json::json!({
                        "connection_id": conn_id,
                    })
                    .as_object()
                    .unwrap()
                    .clone(),
                ),
            )
            .await
            .ok();
    }

    client.cancel().await.ok();
}
