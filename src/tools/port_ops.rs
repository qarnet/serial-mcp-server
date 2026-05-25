use std::collections::HashMap;
use std::sync::Arc;

use rmcp::{service::RequestContext, Json, RoleServer};
use tracing::{debug, info};

use crate::security::SecurityManager;
use crate::serial::{ConnectionManager, PortInfo};
use crate::tools::helpers::log_tool_err;
use crate::tools::helpers::parse_open_args;
use crate::tools::types::{CloseArgs, CloseResult, ListPortsResult, OpenArgs, OpenResult};

use crate::resources::URI_CONNECTION_PREFIX;

pub async fn list_ports() -> Result<Json<ListPortsResult>, String> {
    debug!("Listing serial ports");
    let ports = PortInfo::list_available()
        .map_err(|e| log_tool_err("list_ports", "Failed to list ports", e))?;
    info!("Found {} serial ports", ports.len());
    Ok(Json(ListPortsResult {
        count: ports.len(),
        ports,
    }))
}

pub async fn open(
    connections: &Arc<ConnectionManager>,
    security: &SecurityManager,
    subscribers: &Arc<tokio::sync::Mutex<HashMap<String, usize>>>,
    args: OpenArgs,
    ctx: RequestContext<RoleServer>,
) -> Result<Json<OpenResult>, String> {
    let config = parse_open_args(args)?;
    let port = config.port.clone();
    let baud_rate = config.baud_rate;
    debug!("Opening {} @ {}", port, baud_rate);

    if !security.is_port_allowed(&port) {
        return Err(format!(
            "Port '{port}' is not in the allowlist. Allowed patterns: {}",
            security.allowlist_summary()
        ));
    }

    let connection_id = connections
        .open(config)
        .await
        .map_err(|e| log_tool_err("open", &format!("Failed to open port {port}"), e))?;
    info!("Opened connection {} -> {}", connection_id, port);

    if let Err(e) = ctx.peer.notify_resource_list_changed().await {
        debug!("Failed to notify resource list changed: {e}");
    }

    let conn_uri = format!("{URI_CONNECTION_PREFIX}{connection_id}");
    let subs = subscribers.lock().await;
    let should_notify = subs.get(&conn_uri).is_some_and(|count| *count > 0);
    drop(subs);

    if should_notify {
        if let Err(e) = ctx
            .peer
            .notify_resource_updated(rmcp::model::ResourceUpdatedNotificationParam::new(conn_uri))
            .await
        {
            debug!("Failed to notify resource updated: {e}");
        }
    }

    Ok(Json(OpenResult {
        connection_id,
        port,
        baud_rate,
    }))
}

pub async fn close(
    connections: &Arc<ConnectionManager>,
    subscribers: &Arc<tokio::sync::Mutex<HashMap<String, usize>>>,
    args: CloseArgs,
    ctx: RequestContext<RoleServer>,
) -> Result<Json<CloseResult>, String> {
    debug!("Closing {}", args.connection_id);

    connections.close(&args.connection_id).await.map_err(|e| {
        log_tool_err(
            "close",
            &format!("Failed to close connection {}", args.connection_id),
            e,
        )
    })?;
    info!("Closed connection {}", args.connection_id);

    if let Err(e) = ctx.peer.notify_resource_list_changed().await {
        debug!("Failed to notify resource list changed: {e}");
    }

    let conn_uri = format!("{URI_CONNECTION_PREFIX}{}", args.connection_id);
    let subs = subscribers.lock().await;
    let should_notify = subs.get(&conn_uri).is_some_and(|count| *count > 0);
    drop(subs);

    if should_notify {
        if let Err(e) = ctx
            .peer
            .notify_resource_updated(rmcp::model::ResourceUpdatedNotificationParam::new(conn_uri))
            .await
        {
            debug!("Failed to notify resource updated: {e}");
        }
    }

    Ok(Json(CloseResult {
        connection_id: args.connection_id,
    }))
}
