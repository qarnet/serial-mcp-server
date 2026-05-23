use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Connection already exists: {0}")]
    ConnectionExists(String),

    #[error("Invalid connection ID: {0}")]
    InvalidConnection(String),

    #[error("Invalid baud rate: {0}")]
    InvalidBaudRate(u32),

    #[error("Read timeout")]
    ReadTimeout,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serial port error: {0}")]
    SerialPortError(#[from] serialport::Error),
}

pub type Result<T> = std::result::Result<T, SerialError>;
