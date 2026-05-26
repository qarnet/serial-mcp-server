use thiserror::Error;

#[derive(Debug, Error)]
pub enum SerialError {
    #[error("Failed to open port: {0}")]
    OpenFailed(String),

    #[error("Port already open: {0}")]
    PortAlreadyOpen(String),

    #[error("Connection not found: {0}")]
    ConnectionNotFound(String),

    #[error("Invalid baud rate: {0}")]
    InvalidBaudRate(u32),

    #[error("Read timeout")]
    ReadTimeout,

    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, SerialError>;

impl From<serialport::Error> for SerialError {
    fn from(err: serialport::Error) -> Self {
        SerialError::IoError(std::io::Error::other(err.to_string()))
    }
}
