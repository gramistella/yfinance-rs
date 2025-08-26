use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct NewsEnvelope {
    pub(crate) data: Option<NewsData>,
}

#[derive(Deserialize)]
pub(crate) struct NewsData {
    #[serde(rename = "tickerStream")]
    pub(crate) ticker_stream: Option<TickerStream>,
}

#[derive(Deserialize)]
pub(crate) struct TickerStream {
    pub(crate) stream: Option<Vec<StreamItem>>,
}

#[derive(Deserialize)]
pub(crate) struct StreamItem {
    pub(crate) id: String,
    pub(crate) content: Option<Content>,
    // The python 'ad' check might be for a field at this level.
    pub(crate) ad: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub(crate) struct Content {
    pub(crate) title: Option<String>,
    #[serde(rename = "pubDate")]
    pub(crate) pub_date: Option<String>,
    pub(crate) provider: Option<Provider>,
    #[serde(rename = "canonicalUrl")]
    pub(crate) canonical_url: Option<CanonicalUrl>,
}

#[derive(Deserialize)]
pub(crate) struct Provider {
    #[serde(rename = "displayName")]
    pub(crate) display_name: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct CanonicalUrl {
    pub(crate) url: Option<String>,
}
