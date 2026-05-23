use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serialport::{available_ports, SerialPortInfo, SerialPortType};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::Mutex;
use tokio::time::timeout;
use tokio_serial::{SerialPortBuilderExt, SerialStream};
use uuid::Uuid;

use crate::error::{Result, SerialError};

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum DataBits {
    #[serde(rename = "5")]
    Five,
    #[serde(rename = "6")]
    Six,
    #[serde(rename = "7")]
    Seven,
    #[serde(rename = "8")]
    Eight,
}

impl From<DataBits> for serialport::DataBits {
    fn from(b: DataBits) -> Self {
        match b {
            DataBits::Five => Self::Five,
            DataBits::Six => Self::Six,
            DataBits::Seven => Self::Seven,
            DataBits::Eight => Self::Eight,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum StopBits {
    #[serde(rename = "1")]
    One,
    #[serde(rename = "2")]
    Two,
}

impl From<StopBits> for serialport::StopBits {
    fn from(b: StopBits) -> Self {
        match b {
            StopBits::One => Self::One,
            StopBits::Two => Self::Two,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl From<Parity> for serialport::Parity {
    fn from(p: Parity) -> Self {
        match p {
            Parity::None => Self::None,
            Parity::Odd => Self::Odd,
            Parity::Even => Self::Even,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FlowControl {
    None,
    Software,
    Hardware,
}

impl From<FlowControl> for serialport::FlowControl {
    fn from(f: FlowControl) -> Self {
        match f {
            FlowControl::None => Self::None,
            FlowControl::Software => Self::Software,
            FlowControl::Hardware => Self::Hardware,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    pub port: String,
    pub baud_rate: u32,
    pub data_bits: DataBits,
    pub stop_bits: StopBits,
    pub parity: Parity,
    pub flow_control: FlowControl,
}

#[derive(Debug, Serialize)]
pub struct PortInfo {
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_id: Option<String>,
}

impl PortInfo {
    pub fn list() -> std::result::Result<Vec<PortInfo>, serialport::Error> {
        Ok(available_ports()?
            .into_iter()
            .map(|p| PortInfo {
                hardware_id: hardware_id(&p),
                description: description(&p),
                name: p.port_name,
            })
            .collect())
    }
}

fn hardware_id(p: &SerialPortInfo) -> Option<String> {
    match &p.port_type {
        SerialPortType::UsbPort(info) => {
            Some(format!("USB VID:{:04X} PID:{:04X}", info.vid, info.pid))
        }
        SerialPortType::PciPort => Some("PCI".to_string()),
        SerialPortType::BluetoothPort => Some("Bluetooth".to_string()),
        SerialPortType::Unknown => None,
    }
}

fn description(p: &SerialPortInfo) -> String {
    match &p.port_type {
        SerialPortType::UsbPort(info) => format!(
            "{} {}",
            info.manufacturer.as_deref().unwrap_or("Unknown"),
            info.product.as_deref().unwrap_or("USB Serial Device")
        ),
        SerialPortType::PciPort => "PCI Serial Port".to_string(),
        SerialPortType::BluetoothPort => "Bluetooth Serial Port".to_string(),
        SerialPortType::Unknown => "Serial Port".to_string(),
    }
}

#[derive(Debug)]
pub struct SerialConnection {
    id: String,
    port: String,
    stream: Mutex<SerialStream>,
}

impl SerialConnection {
    pub async fn open(config: ConnectionConfig) -> Result<Self> {
        if config.baud_rate == 0 || config.baud_rate > 4_000_000 {
            return Err(SerialError::InvalidBaudRate(config.baud_rate));
        }

        let stream = tokio_serial::new(&config.port, config.baud_rate)
            .data_bits(config.data_bits.into())
            .stop_bits(config.stop_bits.into())
            .parity(config.parity.into())
            .flow_control(config.flow_control.into())
            .open_native_async()
            .map_err(|e| SerialError::ConnectionFailed(format!("{}: {}", config.port, e)))?;

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            port: config.port,
            stream: Mutex::new(stream),
        })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn port(&self) -> &str {
        &self.port
    }

    pub async fn write(&self, data: &[u8]) -> Result<usize> {
        let mut stream = self.stream.lock().await;
        let n = stream.write(data).await?;
        stream.flush().await?;
        Ok(n)
    }

    pub async fn read(&self, buf: &mut [u8], timeout_ms: Option<u64>) -> Result<usize> {
        let mut stream = self.stream.lock().await;
        match timeout_ms {
            Some(ms) => match timeout(Duration::from_millis(ms), stream.read(buf)).await {
                Ok(r) => Ok(r?),
                Err(_) => Err(SerialError::ReadTimeout),
            },
            None => Ok(stream.read(buf).await?),
        }
    }
}

#[derive(Debug, Default)]
pub struct ConnectionManager {
    connections: Mutex<HashMap<String, Arc<SerialConnection>>>,
}

impl ConnectionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open(&self, config: ConnectionConfig) -> Result<String> {
        let mut conns = self.connections.lock().await;
        if conns.values().any(|c| c.port() == config.port) {
            return Err(SerialError::ConnectionExists(config.port));
        }
        let conn = Arc::new(SerialConnection::open(config).await?);
        let id = conn.id().to_string();
        conns.insert(id.clone(), conn);
        Ok(id)
    }

    pub async fn close(&self, id: &str) -> Result<()> {
        self.connections
            .lock()
            .await
            .remove(id)
            .ok_or_else(|| SerialError::InvalidConnection(id.to_string()))?;
        Ok(())
    }

    pub async fn get(&self, id: &str) -> Result<Arc<SerialConnection>> {
        self.connections
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or_else(|| SerialError::InvalidConnection(id.to_string()))
    }
}
