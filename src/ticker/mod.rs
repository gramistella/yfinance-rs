use crate::{history::HistoryBuilder, internal::net, YfClient, YfError};
use serde::Deserialize;
use url::Url;

const DEFAULT_BASE_QUOTE_V7: &str = "https://query1.finance.yahoo.com/v7/finance/quote";
const DEFAULT_BASE_OPTIONS_V7: &str = "https://query1.finance.yahoo.com/v7/finance/options";

pub struct Ticker<'a> {
    client: &'a mut YfClient,
    symbol: String,
    quote_base: Url,
    options_base: Url,
}

impl<'a> Ticker<'a> {
    pub fn new(client: &'a mut YfClient, symbol: impl Into<String>) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            options_base: Url::parse(DEFAULT_BASE_OPTIONS_V7)?,
        })
    }

    pub fn with_quote_base(
        client: &'a mut YfClient,
        symbol: impl Into<String>,
        base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: base,
            options_base: Url::parse(DEFAULT_BASE_OPTIONS_V7)?,
        })
    }

    pub fn with_options_base(
        client: &'a mut YfClient,
        symbol: impl Into<String>,
        base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base: Url::parse(DEFAULT_BASE_QUOTE_V7)?,
            options_base: base,
        })
    }

    pub fn with_bases(
        client: &'a mut YfClient,
        symbol: impl Into<String>,
        quote_base: Url,
        options_base: Url,
    ) -> Result<Self, YfError> {
        Ok(Self {
            client,
            symbol: symbol.into(),
            quote_base,
            options_base,
        })
    }

    pub fn history_builder(&self) -> HistoryBuilder<'_> {
        HistoryBuilder::new(&*self.client, &self.symbol)
    }

    pub async fn quote(&mut self) -> Result<Quote, YfError> {
        let http = self.client.http().clone();

        let mut url = self.quote_base.clone();
        {
            let mut qp = url.query_pairs_mut();
            qp.append_pair("symbols", &self.symbol);
        }

        let mut resp = http
            .get(url.clone())
            .header("accept", "application/json")
            .send()
            .await?;

        if resp.status().is_success() {
            return self.parse_quote(resp).await;
        }

        let code = resp.status().as_u16();
        if code != 401 && code != 403 {
            return Err(YfError::Status {
                status: code,
                url: url.to_string(),
            });
        }

        self.client.ensure_credentials().await?;
        let crumb = match self.client.crumb() {
            Some(c) => c.to_string(),
            None => {
                return Err(YfError::Status {
                    status: code,
                    url: url.to_string(),
                })
            }
        };

        let mut url2 = self.quote_base.clone();
        {
            let mut qp = url2.query_pairs_mut();
            qp.append_pair("symbols", &self.symbol);
            qp.append_pair("crumb", &crumb);
        }

        resp = http
            .get(url2.clone())
            .header("accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(YfError::Status {
                status: resp.status().as_u16(),
                url: url2.to_string(),
            });
        }

        self.parse_quote(resp).await
    }

    async fn parse_quote(&self, resp: reqwest::Response) -> Result<Quote, YfError> {
        let body = net::get_text(resp, "quote_v7", &self.symbol, "json").await?;
        let env: V7Envelope =
            serde_json::from_str(&body).map_err(|e| YfError::Data(format!("quote json parse: {e}")))?;

        let result = env
            .quote_response
            .and_then(|qr| qr.result)
            .and_then(|mut v| v.pop())
            .ok_or_else(|| YfError::Data("empty quote result".into()))?;

        Ok(Quote {
            symbol: result.symbol.unwrap_or_else(|| self.symbol.clone()),
            regular_market_price: result.regular_market_price,
            regular_market_previous_close: result.regular_market_previous_close,
            currency: result.currency,
            exchange: result
                .full_exchange_name
                .or(result.exchange)
                .or(result.market)
                .or(result.market_cap_figure_exchange),
            market_state: result.market_state,
        })
    }

    pub async fn fast_info(&mut self) -> Result<FastInfo, YfError> {
        let q = self.quote().await?;
        let last = q
            .regular_market_price
            .or(q.regular_market_previous_close)
            .ok_or_else(|| YfError::Data("quote missing last/previous price".into()))?;

        Ok(FastInfo {
            symbol: q.symbol,
            last_price: last,
            previous_close: q.regular_market_previous_close,
            currency: q.currency,
            exchange: q.exchange,
            market_state: q.market_state,
        })
    }

    pub async fn history(
        &self,
        range: Option<crate::Range>,
        interval: Option<crate::Interval>,
        prepost: bool,
    ) -> Result<Vec<crate::Candle>, YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        if let Some(i) = interval {
            hb = hb.interval(i);
        }
        hb = hb.auto_adjust(true).prepost(prepost).actions(true);
        hb.fetch().await
    }

    pub async fn actions(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<crate::Action>, YfError> {
        let mut hb = self.history_builder();
        hb = hb.range(range.unwrap_or(crate::Range::Max));
        let resp = hb.auto_adjust(true).actions(true).fetch_full().await?;
        Ok(resp.actions)
    }

    pub async fn dividends(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, f64)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Dividend { ts, amount } => Some((ts, amount)),
                _ => None,
            })
            .collect())
    }

    pub async fn splits(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Vec<(i64, u32, u32)>, YfError> {
        let acts = self.actions(range).await?;
        Ok(acts
            .into_iter()
            .filter_map(|a| match a {
                crate::Action::Split { ts, numerator, denominator } => Some((ts, numerator, denominator)),
                _ => None,
            })
            .collect())
    }

    pub async fn get_history_metadata(
        &self,
        range: Option<crate::Range>,
    ) -> Result<Option<crate::HistoryMeta>, YfError> {
        let mut hb = self.history_builder();
        if let Some(r) = range {
            hb = hb.range(r);
        }
        let resp = hb.fetch_full().await?;
        Ok(resp.meta)
    }

    /* ---------------- Options API ---------------- */

    pub async fn options(&mut self) -> Result<Vec<i64>, YfError> {
        let (body, _) = self.fetch_options_raw(None).await?;
        let env: OptEnvelope = serde_json::from_str(&body)
            .map_err(|e| YfError::Data(format!("options json parse: {e}")))?;
        let first = env.option_chain
            .and_then(|oc| oc.result)
            .and_then(|mut v| v.pop())
            .ok_or_else(|| YfError::Data("empty options result".into()))?;
        Ok(first.expiration_dates.unwrap_or_default())
    }

    pub async fn option_chain(&mut self, date: Option<i64>) -> Result<OptionChain, YfError> {
        let (body, used_url) = self.fetch_options_raw(date).await?;
        let env: OptEnvelope = serde_json::from_str(&body)
            .map_err(|e| YfError::Data(format!("options json parse: {e}")))?;

        let first = env.option_chain
            .and_then(|oc| oc.result)
            .and_then(|mut v| v.pop())
            .ok_or_else(|| YfError::Data("empty options result".into()))?;

        let od = match first.options.and_then(|mut v| v.pop()) {
            Some(x) => x,
            None => {
                return Ok(OptionChain { calls: vec![], puts: vec![] });
            }
        };

        let expiration = od.expiration_date.unwrap_or_else(|| {
            if let Some(q) = used_url.query() {
                for kv in q.split('&') {
                    if let Some(v) = kv.strip_prefix("date=")
                        && let Ok(ts) = v.parse::<i64>() { return ts }
                }
            }
            0
        });

        let map = |side: Option<Vec<OptContractNode>>| -> Vec<OptionContract> {
            side.unwrap_or_default()
                .into_iter()
                .map(|c| OptionContract {
                    contract_symbol: c.contract_symbol.unwrap_or_default(),
                    strike: c.strike.unwrap_or(0.0),
                    last_price: c.last_price,
                    bid: c.bid,
                    ask: c.ask,
                    volume: c.volume,
                    open_interest: c.open_interest,
                    implied_volatility: c.implied_volatility,
                    in_the_money: c.in_the_money.unwrap_or(false),
                    expiration,
                })
                .collect()
        };

        Ok(OptionChain {
            calls: map(od.calls),
            puts: map(od.puts),
        })
    }

    async fn fetch_options_raw(&mut self, date: Option<i64>) -> Result<(String, Url), YfError> {
        let http = self.client.http().clone();

        let mut url = self.options_base.join(&self.symbol)?;
        {
            let mut qp = url.query_pairs_mut();
            if let Some(d) = date {
                qp.append_pair("date", &d.to_string());
            }
        }

        let mut resp = http
            .get(url.clone())
            .header("accept", "application/json")
            .send()
            .await?;

        if resp.status().is_success() {
            let fixture_symbol = match date {
                Some(d) => format!("{}_{}", self.symbol, d),
                None => self.symbol.clone(),
            };
            let body = net::get_text(resp, "options_v7", &fixture_symbol, "json").await?;
            return Ok((body, url));
        }

        let code = resp.status().as_u16();
        if code != 401 && code != 403 {
            return Err(YfError::Status {
                status: code,
                url: url.to_string(),
            });
        }

        self.client.ensure_credentials().await?;
        let crumb = self.client.crumb().ok_or_else(|| YfError::Status {
            status: code,
            url: url.to_string(),
        })?;

        let mut url2 = self.options_base.join(&self.symbol)?;
        {
            let mut qp = url2.query_pairs_mut();
            if let Some(d) = date {
                qp.append_pair("date", &d.to_string());
            }
            qp.append_pair("crumb", crumb);
        }

        resp = http
            .get(url2.clone())
            .header("accept", "application/json")
            .send()
            .await?;

        if !resp.status().is_success() {
            return Err(YfError::Status {
                status: resp.status().as_u16(),
                url: url2.to_string(),
            });
        }

        let fixture_symbol = match date {
            Some(d) => format!("{}_{}", self.symbol, d),
            None => self.symbol.clone(),
        };
        let body = net::get_text(resp, "options_v7", &fixture_symbol, "json").await?;
        Ok((body, url2))
    }

    /* ---------------- Fundamentals convenience (new) ---------------- */

    pub async fn income_stmt(&mut self) -> Result<Vec<crate::IncomeStatementRow>, YfError> {
        crate::fundamentals::income_statement(self.client, &self.symbol, false).await
    }

    pub async fn quarterly_income_stmt(
        &mut self,
    ) -> Result<Vec<crate::IncomeStatementRow>, YfError> {
        crate::fundamentals::income_statement(self.client, &self.symbol, true).await
    }

    pub async fn balance_sheet(&mut self) -> Result<Vec<crate::BalanceSheetRow>, YfError> {
        crate::fundamentals::balance_sheet(self.client, &self.symbol, false).await
    }

    pub async fn quarterly_balance_sheet(
        &mut self,
    ) -> Result<Vec<crate::BalanceSheetRow>, YfError> {
        crate::fundamentals::balance_sheet(self.client, &self.symbol, true).await
    }

    pub async fn cashflow(&mut self) -> Result<Vec<crate::CashflowRow>, YfError> {
        crate::fundamentals::cashflow(self.client, &self.symbol, false).await
    }

    pub async fn quarterly_cashflow(&mut self) -> Result<Vec<crate::CashflowRow>, YfError> {
        crate::fundamentals::cashflow(self.client, &self.symbol, true).await
    }

    pub async fn earnings(&mut self) -> Result<crate::Earnings, YfError> {
        crate::fundamentals::earnings(self.client, &self.symbol).await
    }

    pub async fn calendar(&mut self) -> Result<crate::FundCalendar, YfError> {
        crate::fundamentals::calendar(self.client, &self.symbol).await
    }
}

/* ---------------- Public models ---------------- */

#[derive(Debug, Clone, PartialEq)]
pub struct Quote {
    pub symbol: String,
    pub regular_market_price: Option<f64>,
    pub regular_market_previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FastInfo {
    pub symbol: String,
    pub last_price: f64,
    pub previous_close: Option<f64>,
    pub currency: Option<String>,
    pub exchange: Option<String>,
    pub market_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionContract {
    pub contract_symbol: String,
    pub strike: f64,
    pub last_price: Option<f64>,
    pub bid: Option<f64>,
    pub ask: Option<f64>,
    pub volume: Option<u64>,
    pub open_interest: Option<u64>,
    pub implied_volatility: Option<f64>,
    pub in_the_money: bool,
    pub expiration: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OptionChain {
    pub calls: Vec<OptionContract>,
    pub puts: Vec<OptionContract>,
}

/* ---------------- Minimal serde mapping for v7 quote ---------------- */

#[derive(Deserialize)]
struct V7Envelope {
    #[serde(rename = "quoteResponse")]
    quote_response: Option<V7QuoteResponse>,
}

#[derive(Deserialize)]
struct V7QuoteResponse {
    result: Option<Vec<V7QuoteNode>>,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct V7QuoteNode {
    #[serde(default)]
    symbol: Option<String>,
    #[serde(rename = "regularMarketPrice")]
    regular_market_price: Option<f64>,
    #[serde(rename = "regularMarketPreviousClose")]
    regular_market_previous_close: Option<f64>,
    currency: Option<String>,

    #[serde(rename = "fullExchangeName")]
    full_exchange_name: Option<String>,
    exchange: Option<String>,
    market: Option<String>,
    #[serde(rename = "marketCapFigureExchange")]
    market_cap_figure_exchange: Option<String>,

    #[serde(rename = "marketState")]
    market_state: Option<String>,
}

/* ---------------- Minimal serde mapping for v7 options ---------------- */

#[derive(Deserialize)]
struct OptEnvelope {
    #[serde(rename = "optionChain")]
    option_chain: Option<OptChainNode>,
}

#[derive(Deserialize)]
struct OptChainNode {
    result: Option<Vec<OptResultNode>>,
    #[allow(dead_code)]
    error: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct OptResultNode {
    #[serde(rename = "expirationDates")]
    expiration_dates: Option<Vec<i64>>,
    options: Option<Vec<OptByDateNode>>,
}

#[derive(Deserialize)]
struct OptByDateNode {
    #[serde(rename = "expirationDate")]
    expiration_date: Option<i64>,
    calls: Option<Vec<OptContractNode>>,
    puts: Option<Vec<OptContractNode>>,
}

#[derive(Deserialize)]
struct OptContractNode {
    #[serde(rename = "contractSymbol")]
    contract_symbol: Option<String>,
    strike: Option<f64>,
    #[serde(rename = "lastPrice")]
    last_price: Option<f64>,
    bid: Option<f64>,
    ask: Option<f64>,
    volume: Option<u64>,
    #[serde(rename = "openInterest")]
    open_interest: Option<u64>,
    #[serde(rename = "impliedVolatility")]
    implied_volatility: Option<f64>,
    #[serde(rename = "inTheMoney")]
    in_the_money: Option<bool>,
}
