use serde::Deserialize;

#[derive(Deserialize)]
pub struct NewsEnvelope {
    pub(crate) data: Option<NewsData>,
}

#[derive(Deserialize)]
pub struct NewsData {
    #[serde(rename = "tickerStream")]
    pub(crate) ticker_stream: Option<TickerStream>,
}

#[derive(Deserialize)]
pub struct TickerStream {
    pub(crate) stream: Option<Vec<StreamItem>>,
}

#[derive(Deserialize)]
pub struct StreamItem {
    pub(crate) id: String,
    pub(crate) content: Option<Content>,
    // The python 'ad' check might be for a field at this level.
    pub(crate) ad: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct Content {
    pub(crate) title: Option<String>,
    #[serde(rename = "pubDate")]
    pub(crate) pub_date: Option<String>,
    pub(crate) provider: Option<Provider>,
    #[serde(rename = "canonicalUrl")]
    pub(crate) canonical_url: Option<CanonicalUrl>,
}

#[derive(Deserialize)]
pub struct Provider {
    #[serde(rename = "displayName")]
    pub(crate) display_name: Option<String>,
}

#[derive(Deserialize)]
pub struct CanonicalUrl {
    pub(crate) url: Option<String>,
}
