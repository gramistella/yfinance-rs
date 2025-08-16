use crate::core::{HistoryResponse, Interval, Range, YfError};

#[derive(Debug, Clone, Copy)]
pub struct HistoryRequest {
    pub range: Option<Range>,
    pub period: Option<(i64, i64)>,
    pub interval: Interval,
    pub include_prepost: bool,
    pub include_actions: bool,
    pub auto_adjust: bool,
    pub keepna: bool,
}

pub trait HistoryService: Send + Sync {
    fn fetch_full_history<'a>(
        &'a self,
        symbol: &'a str,
        req: HistoryRequest,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = Result<HistoryResponse, YfError>> + Send + 'a>,
    >;
}
