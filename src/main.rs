use rmcp::{transport::stdio, ServiceExt};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

use serial_mcp_server::SerialHandler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();

    info!("Starting Serial MCP Server v{}", env!("CARGO_PKG_VERSION"));

    let service = SerialHandler::new().serve(stdio()).await.map_err(|e| {
        error!("Failed to start server: {:?}", e);
        e
    })?;

    info!("Serial MCP Server started");
    service.waiting().await?;
    info!("Serial MCP Server stopped");
    Ok(())
}
