use thiserror::Error;

/// The primary error type for all fallible operations in this crate.
#[derive(Debug, Error)]
pub enum YfError {
    /// An error occurred during an HTTP request.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// An error occurred with the WebSocket connection.
    #[error("WebSocket error: {0}")]
    Websocket(Box<tokio_tungstenite::tungstenite::Error>),

    /// An error occurred while decoding a protobuf message from a stream.
    #[error("Protobuf decoding error: {0}")]
    Protobuf(#[from] prost::DecodeError),

    /// A provided URL could not be parsed.
    #[error("Invalid URL: {0}")]
    Url(#[from] url::ParseError),

    /// The server returned an unexpected or unsuccessful HTTP status code.
    #[error("Unexpected response status: {status} at {url}")]
    Status {
        /// The HTTP status code.
        status: u16,
        /// The URL that returned the error.
        url: String,
    },

    /// The data received from the API was in an unexpected format or was missing a required field.
    #[error("Data format unexpected or missing field: {0}")]
    Data(String),

    /// An invalid date range was provided for a historical data request (start must be before end).
    #[error("invalid date range: start must be before end")]
    InvalidDates,
}

impl From<tokio_tungstenite::tungstenite::Error> for YfError {
    fn from(e: tokio_tungstenite::tungstenite::Error) -> Self {
        YfError::Websocket(Box::new(e))
    }
}
