use std::marker::PhantomData;

use serde_json::{Map, Value};
use url::Url;

use super::fields::{equity_fields, etf_fields, fund_fields};
use super::predefined::PredefinedScreener;
use super::query::{
    Equity, EquityQuery, Etf, EtfQuery, Fund, FundQuery, Predefined, ResultOffset, ScreenerCount,
    ScreenerQuery, SortDirection, SortField, YahooQuoteType,
};
use super::response::{ScreenerResponse, parse_screener_body_with_diagnostics};
use crate::YfResponse;
use crate::core::CallOptions;
use crate::core::client::{CacheEndpoint, CacheMode};
use crate::{YfClient, YfError};

const DEFAULT_SCREENER_BASE: &str = "https://query1.finance.yahoo.com/v1/finance/screener";
const DEFAULT_PREDEFINED_SCREENER_BASE: &str =
    "https://query1.finance.yahoo.com/v1/finance/screener/predefined/saved";

#[derive(Debug, Clone)]
enum RequestKind {
    Predefined(PredefinedScreener),
    Custom {
        query: Value,
        quote_type: YahooQuoteType,
        sort_field: &'static str,
        sort_direction: SortDirection,
    },
}

#[derive(Debug, Clone)]
pub(super) struct CustomParts {
    query: Value,
    quote_type: YahooQuoteType,
    sort_field: &'static str,
    sort_direction: SortDirection,
}

impl CustomParts {
    pub(super) fn equity(
        sort_field: SortField<Equity>,
        sort_direction: SortDirection,
        query: EquityQuery,
    ) -> Self {
        Self {
            query: query.into_wire_value(),
            quote_type: YahooQuoteType::Equity,
            sort_field: sort_field.key(),
            sort_direction,
        }
    }

    pub(super) fn fund(
        sort_field: SortField<Fund>,
        sort_direction: SortDirection,
        query: FundQuery,
    ) -> Self {
        Self {
            query: query.into_wire_value(),
            quote_type: YahooQuoteType::MutualFund,
            sort_field: sort_field.key(),
            sort_direction,
        }
    }

    pub(super) fn etf(
        sort_field: SortField<Etf>,
        sort_direction: SortDirection,
        query: EtfQuery,
    ) -> Self {
        Self {
            query: query.into_wire_value(),
            quote_type: YahooQuoteType::Etf,
            sort_field: sort_field.key(),
            sort_direction,
        }
    }
}

/// Builder for Yahoo Finance screener requests.
#[derive(Debug, Clone)]
pub struct ScreenerBuilder<U = Predefined> {
    client: YfClient,
    screener_base: Url,
    predefined_base: Url,
    kind: RequestKind,
    count: Option<ScreenerCount>,
    offset: Option<ResultOffset>,
    options: CallOptions,
    marker: PhantomData<U>,
}

impl ScreenerBuilder<Predefined> {
    /// Creates a builder for a predefined Yahoo screener.
    ///
    /// # Panics
    ///
    /// Panics if hardcoded Yahoo screener URLs are invalid.
    #[must_use]
    pub fn predefined(client: &YfClient, screener: PredefinedScreener) -> Self {
        Self {
            client: client.clone(),
            screener_base: Url::parse(DEFAULT_SCREENER_BASE).expect("valid screener URL"),
            predefined_base: Url::parse(DEFAULT_PREDEFINED_SCREENER_BASE)
                .expect("valid predefined screener URL"),
            kind: RequestKind::Predefined(screener),
            count: None,
            offset: None,
            options: CallOptions::default(),
            marker: PhantomData,
        }
    }

    /// Alias for [`ScreenerBuilder::predefined`].
    #[must_use]
    pub fn new(client: &YfClient, screener: PredefinedScreener) -> Self {
        Self::predefined(client, screener)
    }
}

impl ScreenerBuilder<Equity> {
    /// Creates a builder for a custom equity screener query.
    ///
    /// # Panics
    ///
    /// Panics if hardcoded Yahoo screener URLs are invalid.
    #[must_use]
    pub fn equity(client: &YfClient, query: EquityQuery) -> Self {
        Self::custom(
            client,
            query,
            YahooQuoteType::Equity,
            equity_fields::TICKER.key(),
        )
    }

    /// Sets the custom equity sort field and direction.
    #[must_use]
    pub const fn sort_by(mut self, field: SortField<Equity>, direction: SortDirection) -> Self {
        self.set_sort(field.key(), direction);
        self
    }
}

impl ScreenerBuilder<Fund> {
    /// Creates a builder for a custom mutual fund screener query.
    ///
    /// # Panics
    ///
    /// Panics if hardcoded Yahoo screener URLs are invalid.
    #[must_use]
    pub fn fund(client: &YfClient, query: FundQuery) -> Self {
        Self::custom(
            client,
            query,
            YahooQuoteType::MutualFund,
            fund_fields::TICKER.key(),
        )
    }

    /// Sets the custom mutual fund sort field and direction.
    #[must_use]
    pub const fn sort_by(mut self, field: SortField<Fund>, direction: SortDirection) -> Self {
        self.set_sort(field.key(), direction);
        self
    }
}

impl ScreenerBuilder<Etf> {
    /// Creates a builder for a custom ETF screener query.
    ///
    /// # Panics
    ///
    /// Panics if hardcoded Yahoo screener URLs are invalid.
    #[must_use]
    pub fn etf(client: &YfClient, query: EtfQuery) -> Self {
        Self::custom(client, query, YahooQuoteType::Etf, etf_fields::TICKER.key())
    }

    /// Sets the custom ETF sort field and direction.
    #[must_use]
    pub const fn sort_by(mut self, field: SortField<Etf>, direction: SortDirection) -> Self {
        self.set_sort(field.key(), direction);
        self
    }
}

impl<U: Send + Sync> ScreenerBuilder<U> {
    fn custom(
        client: &YfClient,
        query: ScreenerQuery<U>,
        quote_type: YahooQuoteType,
        default_sort_field: &'static str,
    ) -> Self {
        Self {
            client: client.clone(),
            screener_base: Url::parse(DEFAULT_SCREENER_BASE).expect("valid screener URL"),
            predefined_base: Url::parse(DEFAULT_PREDEFINED_SCREENER_BASE)
                .expect("valid predefined screener URL"),
            kind: RequestKind::Custom {
                query: query.into_wire_value(),
                quote_type,
                sort_field: default_sort_field,
                sort_direction: SortDirection::Desc,
            },
            count: Some(ScreenerCount::DEFAULT),
            offset: Some(ResultOffset::ZERO),
            options: CallOptions::default(),
            marker: PhantomData,
        }
    }

    /// Overrides the base URL for custom screener POST requests.
    #[must_use]
    pub fn screener_base(mut self, base: Url) -> Self {
        self.screener_base = base;
        self
    }

    /// Overrides the base URL for predefined screener GET requests.
    #[must_use]
    pub fn predefined_screener_base(mut self, base: Url) -> Self {
        self.predefined_base = base;
        self
    }

    crate::core::impl_call_option_setters!(
        strict_doc = "Fails when Yahoo screener data cannot be projected losslessly."
    );

    /// Sets the requested row count.
    #[must_use]
    pub const fn count(mut self, count: ScreenerCount) -> Self {
        self.count = Some(count);
        self
    }

    /// Sets the result offset.
    #[must_use]
    pub const fn offset(mut self, offset: ResultOffset) -> Self {
        self.offset = Some(offset);
        self
    }

    const fn set_sort(&mut self, field: &'static str, direction: SortDirection) {
        if let RequestKind::Custom {
            sort_field,
            sort_direction,
            ..
        } = &mut self.kind
        {
            *sort_field = field;
            *sort_direction = direction;
        }
    }

    /// Executes the screener request.
    ///
    /// # Errors
    ///
    /// Returns `YfError` if the network request fails, Yahoo returns an error
    /// status, or the response cannot be parsed.
    pub async fn fetch(&self) -> Result<ScreenerResponse, YfError> {
        Ok(self.fetch_with_diagnostics().await?.into_data())
    }

    /// Executes the screener request with projection diagnostics.
    ///
    /// # Errors
    ///
    /// Returns `YfError` if the network request fails, Yahoo returns an error
    /// status, the response cannot be parsed, or strict data-quality mode rejects a projection issue.
    pub async fn fetch_with_diagnostics(&self) -> Result<YfResponse<ScreenerResponse>, YfError> {
        match self.kind.clone() {
            RequestKind::Predefined(screener) if self.offset.is_none() => {
                self.fetch_predefined_get(screener).await
            }
            RequestKind::Predefined(screener) => {
                let parts = screener.custom_parts()?;
                self.fetch_custom_post(parts).await
            }
            RequestKind::Custom {
                query,
                quote_type,
                sort_field,
                sort_direction,
            } => {
                self.fetch_custom_post(CustomParts {
                    query,
                    quote_type,
                    sort_field,
                    sort_direction,
                })
                .await
            }
        }
    }

    async fn fetch_predefined_get(
        &self,
        screener: PredefinedScreener,
    ) -> Result<YfResponse<ScreenerResponse>, YfError> {
        let mut url = self.predefined_base.clone();
        append_common_params(&mut url);
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("scrIds", screener.id());
            if let Some(count) = self.count {
                qp.append_pair("count", &count.get().to_string());
            }
        }

        if self.options.cache_mode().reads(CacheEndpoint::Screener)
            && let Some(body) = self.client.cache_get(&url)
        {
            return parse_screener_body_with_diagnostics(&body, self.options.data_quality());
        }

        let cache_url = url.clone();
        let (body, _) = self.get_with_auth_retry(url, screener.id()).await?;
        let response = parse_screener_body_with_diagnostics(&body, self.options.data_quality())?;
        if self.options.cache_mode().writes(CacheEndpoint::Screener) {
            self.client
                .cache_put(CacheEndpoint::Screener, &cache_url, &body, None);
        }
        Ok(response)
    }

    async fn fetch_custom_post(
        &self,
        parts: CustomParts,
    ) -> Result<YfResponse<ScreenerResponse>, YfError> {
        let mut url = self.screener_base.clone();
        append_common_params(&mut url);

        let fixture_key = parts.quote_type.as_str().to_ascii_lowercase();
        let body = self.custom_body(parts);
        let body_json = serde_json::to_string(&body).map_err(YfError::json)?;
        let response = self
            .post_with_auth_retry(url, &body_json, &fixture_key)
            .await?;
        parse_screener_body_with_diagnostics(&response, self.options.data_quality())
    }

    fn custom_body(&self, parts: CustomParts) -> Value {
        let mut body = Map::new();

        if let Some(offset) = self.offset {
            body.insert("offset".into(), Value::from(offset.get()));
        }
        if let Some(count) = self.count {
            body.insert("count".into(), Value::from(count.get()));
        }

        body.insert(
            "sortField".into(),
            Value::String(parts.sort_field.to_string()),
        );
        body.insert(
            "sortType".into(),
            Value::String(parts.sort_direction.as_str().to_string()),
        );
        body.insert("userId".into(), Value::String(String::new()));
        body.insert("userIdType".into(), Value::String("guid".into()));
        body.insert(
            "quoteType".into(),
            Value::String(parts.quote_type.as_str().into()),
        );
        body.insert("query".into(), parts.query);

        Value::Object(body)
    }

    async fn get_with_auth_retry(
        &self,
        url: Url,
        fixture_key: &str,
    ) -> Result<(String, Url), YfError> {
        let options = self.options.clone().with_cache_mode(CacheMode::Bypass);
        crate::core::net::fetch_text_with_auth_retry(
            &self.client,
            url,
            crate::core::net::AuthFetchConfig {
                auth_mode: crate::core::net::AuthMode::OptionalCrumb,
                cache_endpoint: CacheEndpoint::Screener,
                options: &options,
                cache_body: None,
                endpoint: "screener_predefined",
                fixture_key,
                ext: "json",
                retry_on_invalid_crumb_body: true,
                cache_validator: Some(super::response::validate_screener_body),
            },
            |url| {
                self.client
                    .http()
                    .get(url)
                    .header("accept", "application/json")
            },
        )
        .await
    }

    async fn post_with_auth_retry(
        &self,
        url: Url,
        body_json: &str,
        fixture_key: &str,
    ) -> Result<String, YfError> {
        let (body, _) = crate::core::net::fetch_text_with_auth_retry(
            &self.client,
            url,
            crate::core::net::AuthFetchConfig {
                auth_mode: crate::core::net::AuthMode::OptionalCrumb,
                cache_endpoint: CacheEndpoint::Screener,
                options: &self.options,
                cache_body: Some(body_json),
                endpoint: "screener_custom",
                fixture_key,
                ext: "json",
                retry_on_invalid_crumb_body: true,
                cache_validator: Some(super::response::validate_screener_body),
            },
            |url| {
                self.client
                    .http()
                    .post(url)
                    .header("accept", "application/json")
                    .header("content-type", "application/json")
                    .body(body_json.to_owned())
            },
        )
        .await?;

        Ok(body)
    }
}

fn append_common_params(url: &mut Url) {
    let mut qp = url.query_pairs_mut();
    qp.append_pair("corsDomain", "finance.yahoo.com");
    qp.append_pair("formatted", "false");
    qp.append_pair("lang", "en-US");
    qp.append_pair("region", "US");
}

#[cfg(test)]
mod tests {
    use paft::Decimal;
    use serde_json::json;

    use super::*;
    use crate::screener::{PercentPoints, Region, equity_fields};

    #[test]
    fn custom_body_matches_python_wire_shape() {
        let client = YfClient::default();
        let query = EquityQuery::and(vec![
            equity_fields::PERCENT_CHANGE.gt(PercentPoints::new(Decimal::new(30, 1))),
            equity_fields::REGION.eq(Region::Us),
        ])
        .unwrap();
        let builder = ScreenerBuilder::equity(&client, query);
        let body = match &builder.kind {
            RequestKind::Custom {
                query,
                quote_type,
                sort_field,
                sort_direction,
            } => builder.custom_body(CustomParts {
                query: query.clone(),
                quote_type: *quote_type,
                sort_field,
                sort_direction: *sort_direction,
            }),
            RequestKind::Predefined(_) => unreachable!(),
        };

        assert_eq!(
            body,
            json!({
                "offset": 0,
                "count": 25,
                "sortField": "ticker",
                "sortType": "DESC",
                "userId": "",
                "userIdType": "guid",
                "quoteType": "EQUITY",
                "query": {
                    "operator": "AND",
                    "operands": [
                        {"operator": "GT", "operands": ["percentchange", 3.0]},
                        {"operator": "EQ", "operands": ["region", "us"]}
                    ]
                }
            })
        );
    }
}
