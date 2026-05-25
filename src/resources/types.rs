use schemars::JsonSchema;
use serde::Serialize;

use crate::serial::ConnectionSummary;

#[derive(Debug, Serialize, JsonSchema)]
pub struct ConnectionsResource {
    #[schemars(schema_with = "crate::schema_helpers::uint_schema")]
    pub count: usize,
    pub connections: Vec<ConnectionSummary>,
}
