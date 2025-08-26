mod builder;
mod wire;

pub use builder::HistoryBuilder;

use crate::core::{HistoryRequest, HistoryResponse, HistoryService, YfClient, YfError};
use core::future::Future;
use core::pin::Pin;

impl HistoryService for YfClient {
    fn fetch_full_history<'a>(
        &'a self,
        symbol: &'a str,
        req: HistoryRequest,
    ) -> Pin<Box<dyn Future<Output = Result<HistoryResponse, YfError>> + Send + 'a>> {
        // Own everything the async block needs:
        let client = self.clone(); // YfClient: Clone
        let symbol = symbol.to_owned(); // own the symbol
        Box::pin(async move {
            // HistoryBuilder::new(&YfClient, impl Into<String>) clones internally,
            // so passing &client here is fine.
            let mut hb = builder::HistoryBuilder::new(&client, &symbol)
                .interval(req.interval)
                .auto_adjust(req.auto_adjust)
                .prepost(req.include_prepost)
                .actions(req.include_actions)
                .keepna(req.keepna);

            if let Some((p1, p2)) = req.period {
                use chrono::{TimeZone, Utc};
                let start = Utc
                    .timestamp_opt(p1, 0)
                    .single()
                    .ok_or(YfError::Data("invalid period1".into()))?;
                let end = Utc
                    .timestamp_opt(p2, 0)
                    .single()
                    .ok_or(YfError::Data("invalid period2".into()))?;
                hb = hb.between(start, end);
            } else if let Some(r) = req.range {
                hb = hb.range(r);
            }

            hb.fetch_full().await
        })
    }
}
