use std::collections::HashMap;
use std::sync::Arc;

use rmcp::{service::RequestContext, Json, RoleServer};
use tracing::{debug, info};

use crate::serial::ConnectionManager;
use crate::tools::helpers::{lookup_connection, parse_encoding, stream_rx};
use crate::tools::types::{SubscribeArgs, SubscribeResult, UnsubscribeArgs, UnsubscribeResult};

/// RAII wrapper around a streaming task. Aborts the task on drop.
pub struct StreamHandle {
    join: tokio::task::JoinHandle<()>,
}

impl Drop for StreamHandle {
    fn drop(&mut self) {
        self.join.abort();
    }
}

pub async fn subscribe(
    connections: &Arc<ConnectionManager>,
    streams: &Arc<tokio::sync::Mutex<HashMap<String, StreamHandle>>>,
    args: SubscribeArgs,
    ctx: RequestContext<RoleServer>,
) -> Result<Json<SubscribeResult>, String> {
    debug!(
        "subscribe {} encoding={} chunk={} poll={}",
        args.connection_id, args.encoding, args.max_chunk_bytes, args.poll_interval_ms
    );

    let encoding = parse_encoding(&args.encoding)?;
    let connection = lookup_connection(connections, &args.connection_id).await?;
    let peer = ctx.peer.clone();

    let id = args.connection_id.clone();
    let chunk_bytes = args.max_chunk_bytes;
    let poll_ms = args.poll_interval_ms;
    let join = tokio::spawn(stream_rx(peer, connection, encoding, chunk_bytes, poll_ms));

    let mut streams = streams.lock().await;
    let replaced_previous = streams.insert(id.clone(), StreamHandle { join }).is_some();
    info!(
        "subscribed RX stream for {} (replaced={})",
        id, replaced_previous
    );

    Ok(Json(SubscribeResult {
        connection_id: id,
        encoding: encoding.to_string(),
        max_chunk_bytes: chunk_bytes,
        poll_interval_ms: poll_ms,
        replaced_previous,
    }))
}

pub async fn unsubscribe(
    streams: &Arc<tokio::sync::Mutex<HashMap<String, StreamHandle>>>,
    args: UnsubscribeArgs,
) -> Result<Json<UnsubscribeResult>, String> {
    debug!("unsubscribe {}", args.connection_id);

    let mut streams = streams.lock().await;
    let was_active = streams.remove(&args.connection_id).is_some();
    info!(
        "unsubscribed {} (was_active={})",
        args.connection_id, was_active
    );

    Ok(Json(UnsubscribeResult {
        connection_id: args.connection_id,
        was_active,
    }))
}
