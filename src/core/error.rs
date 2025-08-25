use thiserror::Error;

#[derive(Debug, Error)]
pub enum YfError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    Websocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("Protobuf decoding error: {0}")]
    Protobuf(#[from] prost::DecodeError),

    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    #[error("Unexpected response status: {status} at {url}")]
    Status { status: u16, url: String },

    #[error("Data format unexpected or missing field: {0}")]
    Data(String),

    #[error("invalid date range: start must be before end")]
    InvalidDates,
}