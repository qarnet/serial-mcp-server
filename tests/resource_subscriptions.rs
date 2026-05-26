//! Layer 2 — Resource subscription tests using the in-process HTTP harness.
//!
//! These tests exercise the MCP resource subscribe/unsubscribe protocol
//! methods against the `SerialHandler`. No child processes or OS serial
//! ports are involved.

use rmcp::model::{SubscribeRequestParams, UnsubscribeRequestParams};

mod common;
use common::{connect_client, TestServer};

#[tokio::test]
async fn resource_subscription_works() {
    let server = TestServer::start().await;
    let (client, _rx) = connect_client(&server).await.unwrap();

    client
        .peer()
        .subscribe(SubscribeRequestParams::new("serial://ports"))
        .await
        .expect("subscribe to ports resource");

    client
        .peer()
        .unsubscribe(UnsubscribeRequestParams::new("serial://ports"))
        .await
        .expect("unsubscribe from ports resource");

    client.cancel().await.ok();
}

#[tokio::test]
async fn resource_subscribe_unsubscribe_roundtrip() {
    let server = TestServer::start().await;
    let (client, _rx) = connect_client(&server).await.unwrap();

    client
        .peer()
        .subscribe(SubscribeRequestParams::new("serial://connections"))
        .await
        .expect("subscribe");

    client
        .peer()
        .subscribe(SubscribeRequestParams::new("serial://connections"))
        .await
        .expect("subscribe again");

    client
        .peer()
        .unsubscribe(UnsubscribeRequestParams::new("serial://connections"))
        .await
        .expect("unsubscribe");

    client.cancel().await.ok();
}
