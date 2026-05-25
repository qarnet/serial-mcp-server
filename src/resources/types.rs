use schemars::JsonSchema;
use serde::Serialize;

use crate::serial::ConnectionSummary;

#[derive(Debug, Serialize, JsonSchema)]
pub struct ConnectionsResource {
    pub count: usize,
    pub connections: Vec<ConnectionSummary>,
}
