use std::future::Future;
use std::sync::Arc;

use base64::{engine::general_purpose, Engine as _};
use rmcp::{
    handler::server::{router::tool::ToolRouter, tool::Parameters},
    model::*,
    service::RequestContext,
    tool, tool_handler, tool_router, ErrorData as McpError, RoleServer, ServerHandler,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::{debug, error, info};

use crate::error::SerialError;
use crate::serial::{
    ConnectionConfig, ConnectionManager, DataBits, FlowControl, Parity, PortInfo, StopBits,
};

#[derive(Debug, Deserialize, JsonSchema)]
pub struct OpenArgs {
    pub port: String,
    pub baud_rate: u32,
    #[serde(default = "default_data_bits")]
    pub data_bits: String,
    #[serde(default = "default_stop_bits")]
    pub stop_bits: String,
    #[serde(default = "default_parity")]
    pub parity: String,
    #[serde(default = "default_flow_control")]
    pub flow_control: String,
}

fn default_data_bits() -> String { "8".into() }
fn default_stop_bits() -> String { "1".into() }
fn default_parity() -> String { "none".into() }
fn default_flow_control() -> String { "none".into() }
fn default_encoding() -> String { "utf8".into() }
fn default_max_bytes() -> usize { 1024 }

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CloseArgs {
    pub connection_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct WriteArgs {
    pub connection_id: String,
    pub data: String,
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ReadArgs {
    pub connection_id: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default = "default_max_bytes")]
    pub max_bytes: usize,
    #[serde(default = "default_encoding")]
    pub encoding: String,
}

impl From<OpenArgs> for ConnectionConfig {
    fn from(a: OpenArgs) -> Self {
        let data_bits = match a.data_bits.as_str() {
            "5" => DataBits::Five,
            "6" => DataBits::Six,
            "7" => DataBits::Seven,
            _ => DataBits::Eight,
        };
        let stop_bits = match a.stop_bits.as_str() {
            "2" => StopBits::Two,
            _ => StopBits::One,
        };
        let parity = match a.parity.to_lowercase().as_str() {
            "odd" => Parity::Odd,
            "even" => Parity::Even,
            _ => Parity::None,
        };
        let flow_control = match a.flow_control.to_lowercase().as_str() {
            "software" => FlowControl::Software,
            "hardware" => FlowControl::Hardware,
            _ => FlowControl::None,
        };
        ConnectionConfig {
            port: a.port,
            baud_rate: a.baud_rate,
            data_bits,
            stop_bits,
            parity,
            flow_control,
        }
    }
}

#[derive(Clone)]
pub struct SerialHandler {
    connections: Arc<ConnectionManager>,
    tool_router: ToolRouter<SerialHandler>,
}

#[tool_router]
impl SerialHandler {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(ConnectionManager::new()),
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "List all available serial ports on the system")]
    async fn list_ports(&self) -> Result<CallToolResult, McpError> {
        debug!("Listing serial ports");
        let ports = PortInfo::list().map_err(|e| {
            error!("list_ports failed: {}", e);
            McpError::internal_error(format!("Failed to list ports: {}", e), None)
        })?;
        info!("Found {} serial ports", ports.len());

        let msg = if ports.is_empty() {
            "No serial ports found on the system".to_string()
        } else {
            let lines: Vec<String> = ports
                .iter()
                .map(|p| match &p.hardware_id {
                    Some(hw) => format!("- {}: {} ({})", p.name, p.description, hw),
                    None => format!("- {}: {}", p.name, p.description),
                })
                .collect();
            format!("Found {} serial ports:\n{}", ports.len(), lines.join("\n"))
        };
        Ok(CallToolResult::success(vec![Content::text(msg)]))
    }

    #[tool(description = "Open a serial port connection with specified configuration")]
    async fn open(&self, Parameters(args): Parameters<OpenArgs>) -> Result<CallToolResult, McpError> {
        let config: ConnectionConfig = args.into();
        let port = config.port.clone();
        let baud = config.baud_rate;
        debug!("Opening {} @ {}", port, baud);

        let id = self.connections.open(config).await.map_err(|e| {
            error!("open {} failed: {}", port, e);
            McpError::internal_error(format!("Error: Failed to open port {} - {}", port, e), None)
        })?;
        info!("Opened connection {} -> {}", id, port);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Serial connection opened\nConnection ID: {}\nPort: {}\nBaud rate: {}",
            id, port, baud
        ))]))
    }

    #[tool(description = "Close an open serial port connection")]
    async fn close(&self, Parameters(args): Parameters<CloseArgs>) -> Result<CallToolResult, McpError> {
        debug!("Closing {}", args.connection_id);
        self.connections.close(&args.connection_id).await.map_err(|e| {
            error!("close {} failed: {}", args.connection_id, e);
            McpError::internal_error(
                format!("Error: Failed to close connection {} - {}", args.connection_id, e),
                None,
            )
        })?;
        info!("Closed connection {}", args.connection_id);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Serial connection closed\nConnection ID: {}",
            args.connection_id
        ))]))
    }

    #[tool(description = "Write data to a serial port connection")]
    async fn write(&self, Parameters(args): Parameters<WriteArgs>) -> Result<CallToolResult, McpError> {
        debug!("Write to {} ({})", args.connection_id, args.encoding);
        let conn = self.connections.get(&args.connection_id).await.map_err(|_| {
            McpError::internal_error(
                format!("Error: Connection ID {} not found", args.connection_id),
                None,
            )
        })?;
        let bytes = decode_data(&args.data, &args.encoding)
            .map_err(|e| McpError::internal_error(format!("Error: Data decoding failed - {}", e), None))?;

        let n = conn.write(&bytes).await.map_err(|e| {
            error!("write to {} failed: {}", args.connection_id, e);
            McpError::internal_error(format!("Error: Data sending failed - {}", e), None)
        })?;
        debug!("Wrote {} bytes to {}", n, args.connection_id);
        Ok(CallToolResult::success(vec![Content::text(format!(
            "Data sent successfully\nConnection ID: {}\nBytes written: {}\nData: {:?}",
            args.connection_id, n, args.data
        ))]))
    }

    #[tool(description = "Read data from a serial port connection")]
    async fn read(&self, Parameters(args): Parameters<ReadArgs>) -> Result<CallToolResult, McpError> {
        debug!("Read from {} (timeout {:?})", args.connection_id, args.timeout_ms);
        let conn = self.connections.get(&args.connection_id).await.map_err(|_| {
            McpError::internal_error(
                format!("Error: Connection ID {} not found", args.connection_id),
                None,
            )
        })?;

        let mut buf = vec![0u8; args.max_bytes];
        match conn.read(&mut buf, args.timeout_ms).await {
            Ok(n) => {
                buf.truncate(n);
                let encoded = encode_data(&buf, &args.encoding).map_err(|e| {
                    McpError::internal_error(format!("Error: Data encoding failed - {}", e), None)
                })?;
                let msg = if n > 0 {
                    format!(
                        "Data read successfully\nConnection ID: {}\nBytes read: {}\nData: {:?}",
                        args.connection_id, n, encoded
                    )
                } else {
                    format!(
                        "Read timeout\nConnection ID: {}\nTimeout: {}ms\nBytes read: 0",
                        args.connection_id,
                        args.timeout_ms.unwrap_or(1000)
                    )
                };
                Ok(CallToolResult::success(vec![Content::text(msg)]))
            }
            Err(SerialError::ReadTimeout) => Ok(CallToolResult::success(vec![Content::text(
                format!(
                    "Read timeout\nConnection ID: {}\nTimeout: {}ms\nBytes read: 0",
                    args.connection_id,
                    args.timeout_ms.unwrap_or(1000)
                ),
            )])),
            Err(e) => {
                error!("read from {} failed: {}", args.connection_id, e);
                Err(McpError::internal_error(
                    format!("Error: Data reading failed - {}", e),
                    None,
                ))
            }
        }
    }
}

impl Default for SerialHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_handler]
impl ServerHandler for SerialHandler {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation::from_build_env(),
            instructions: Some(
                "A serial port communication MCP server. Use list_ports to discover available serial ports, then open connections to communicate with serial devices.".into(),
            ),
        }
    }

    async fn initialize(
        &self,
        _req: InitializeRequestParam,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, McpError> {
        info!("Serial MCP server initialized");
        Ok(self.get_info())
    }
}

fn decode_data(data: &str, encoding: &str) -> Result<Vec<u8>, String> {
    match encoding.to_lowercase().as_str() {
        "utf8" | "utf-8" => Ok(data.as_bytes().to_vec()),
        "hex" => {
            let s = data.trim().replace(' ', "");
            if !s.len().is_multiple_of(2) {
                return Err("Hex string must have even length".into());
            }
            hex::decode(&s).map_err(|e| format!("Invalid hex: {}", e))
        }
        "base64" => general_purpose::STANDARD
            .decode(data.trim())
            .or_else(|_| general_purpose::URL_SAFE_NO_PAD.decode(data.trim()))
            .map_err(|e| format!("Invalid base64: {}", e)),
        _ => Err(format!("Unsupported encoding: {}", encoding)),
    }
}

fn encode_data(data: &[u8], encoding: &str) -> Result<String, String> {
    match encoding.to_lowercase().as_str() {
        "utf8" | "utf-8" => {
            String::from_utf8(data.to_vec()).map_err(|e| format!("Invalid UTF-8: {}", e))
        }
        "hex" => Ok(data.iter().map(|b| format!("{:02x}", b)).collect::<Vec<_>>().join(" ")),
        "base64" => Ok(general_purpose::STANDARD.encode(data)),
        _ => Err(format!("Unsupported encoding: {}", encoding)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_roundtrip() {
        let bytes = decode_data("Hello, 世界!", "utf8").unwrap();
        assert_eq!(encode_data(&bytes, "utf8").unwrap(), "Hello, 世界!");
    }

    #[test]
    fn hex_roundtrip() {
        assert_eq!(decode_data("48656c6c6f", "hex").unwrap(), b"Hello");
        assert_eq!(decode_data("48 65 6c 6c 6f", "hex").unwrap(), b"Hello");
        assert_eq!(decode_data("48656C6C6F", "hex").unwrap(), b"Hello");
        assert_eq!(encode_data(b"Hello", "hex").unwrap(), "48 65 6c 6c 6f");
    }

    #[test]
    fn hex_invalid() {
        assert!(decode_data("48656c6c6", "hex").is_err());
        assert!(decode_data("48656cXY", "hex").is_err());
    }

    #[test]
    fn base64_roundtrip() {
        assert_eq!(decode_data("SGVsbG8gV29ybGQ=", "base64").unwrap(), b"Hello World");
        assert_eq!(decode_data("SGVsbG8gV29ybGQ", "base64").unwrap(), b"Hello World");
        assert_eq!(encode_data(b"Hello World", "base64").unwrap(), "SGVsbG8gV29ybGQ=");
    }

    #[test]
    fn unsupported_encoding() {
        assert!(decode_data("test", "unknown").is_err());
        assert!(encode_data(b"test", "unknown").is_err());
    }

    #[test]
    fn binary_roundtrip() {
        let data = b"Hello, World! 123 \x00\xFF";
        let hex = encode_data(data, "hex").unwrap();
        assert_eq!(decode_data(&hex, "hex").unwrap(), data);
        let b64 = encode_data(data, "base64").unwrap();
        assert_eq!(decode_data(&b64, "base64").unwrap(), data);
    }
}
