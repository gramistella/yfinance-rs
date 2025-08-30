use crate::core::{HistoryResponse, Interval, Range, YfError};

/// Encapsulates all parameters for a single historical data request.
///
/// This struct is used as a generic way to request historical data, decoupling modules
/// like `download` from the specific implementation details of `history::HistoryBuilder`.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct HistoryRequest {
    /// A relative time range for the request (e.g., `1y`, `6mo`).
    ///
    /// If `Some`, this takes precedence over `period`.
    pub range: Option<Range>,
    /// An absolute time period for the request, specified as `(start, end)` Unix timestamps.
    pub period: Option<(i64, i64)>,
    /// The time interval for each data point (candle).
    pub interval: Interval,
    /// Whether to include pre-market and post-market data for intraday intervals.
    pub include_prepost: bool,
    /// Whether to include corporate actions (dividends and splits) in the response.
    pub include_actions: bool,
    /// Whether to automatically adjust prices for splits and dividends.
    pub auto_adjust: bool,
    /// Whether to keep data rows that have missing OHLC values.
    pub keepna: bool,
}

/// A trait for services that can fetch historical financial data.
///
/// This allows for abstracting the history fetching logic, making it easier to test
/// and decoupling different parts of the crate. It is implemented by [`YfClient`].
pub trait HistoryService: Send + Sync {
    /// Asynchronously fetches the complete historical data for a given symbol and request.
    ///
    /// # Arguments
    /// * `symbol` - The ticker symbol to fetch data for.
    /// * `req` - A `HistoryRequest` struct containing all the parameters for the query.
    ///
    /// # Returns
    /// A `Future` that resolves to a `Result` containing either a `HistoryResponse` on success
    /// or a `YfError` on failure.
    fn fetch_full_history<'a>(
        &'a self,
        symbol: &'a str,
        req: HistoryRequest,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<HistoryResponse, YfError>> + Send + 'a>,
    >;
}
