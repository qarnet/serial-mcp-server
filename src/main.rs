use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{transport::stdio, ServiceExt};
use serial_mcp_server::security::SecurityManager;
use serial_mcp_server::serial::ConnectionManager;
use serial_mcp_server::server::StreamRegistry;
use serial_mcp_server::SerialHandler;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

const DEFAULT_HTTP_BIND: &str = "127.0.0.1:8000";
const ENV_HTTP_BIND: &str = "SERIAL_MCP_HTTP_BIND";
const ENV_TRANSPORT: &str = "SERIAL_MCP_TRANSPORT";
const MOUNT_PATH: &str = "/mcp";

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .with_target(true)
        .init();
}

fn use_http_transport() -> bool {
    let mut args = std::env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--transport=http" => return true,
            "--transport" => {
                if args.next().map(|v| v == "http").unwrap_or(false) {
                    return true;
                }
            }
            _ => {}
        }
    }
    std::env::var(ENV_TRANSPORT)
        .map(|v| v == "http")
        .unwrap_or(false)
}

async fn run_stdio() -> Result<(), Box<dyn std::error::Error>> {
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

async fn run_http() -> Result<(), Box<dyn std::error::Error>> {
    let bind = std::env::var(ENV_HTTP_BIND).unwrap_or_else(|_| DEFAULT_HTTP_BIND.to_string());
    info!(
        "Starting Serial MCP Server (HTTP) v{} on http://{}{}",
        env!("CARGO_PKG_VERSION"),
        bind,
        MOUNT_PATH
    );

    let shutdown = tokio_util::sync::CancellationToken::new();
    let manager = Arc::new(ConnectionManager::new());
    let streams: StreamRegistry = Arc::new(tokio::sync::Mutex::new(HashMap::new()));
    let security = SecurityManager::from_env();
    let manager_for_service = Arc::clone(&manager);
    let streams_for_service = Arc::clone(&streams);

    let service = StreamableHttpService::new(
        move || {
            Ok(SerialHandler::with_manager_security_and_streams(
                Arc::clone(&manager_for_service),
                security.clone(),
                Arc::clone(&streams_for_service),
            ))
        },
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default().with_cancellation_token(shutdown.child_token()),
    );

    let router = axum::Router::new().nest_service(MOUNT_PATH, service);
    let listener = tokio::net::TcpListener::bind(&bind).await.map_err(|e| {
        error!("Failed to bind {}: {}", bind, e);
        e
    })?;

    let server_shutdown = shutdown.clone();
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            if tokio::signal::ctrl_c().await.is_ok() {
                info!("Ctrl-C received, shutting down");
            }
            server_shutdown.cancel();
        })
        .await?;

    info!("Serial MCP Server (HTTP) stopped");
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracing();
    if use_http_transport() {
        run_http().await
    } else {
        run_stdio().await
    }
}
