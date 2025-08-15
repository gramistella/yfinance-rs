use yfinance_rs::YfClient;

mod common;
use common::{write_fixture};

async fn record_symbol(client: &mut YfClient, sym: &str) -> anyhow::Result<()> {
    // Ensure cookie + crumb are set
    client.ensure_credentials().await?;

    // Persist cookie + crumb once (global)
    if let Some(cookie) = client.cookie() {
        write_fixture("auth/cookie.txt", cookie);
    }
    if let Some(crumb) = client.crumb() {
        write_fixture("auth/crumb.txt", crumb);
    }

    // 1) quoteSummary API (JSON)
    let mut url = client.base_quote_api().join(sym)?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("modules", "assetProfile,quoteType,fundProfile");
        if let Some(crumb) = client.crumb() {
            qp.append_pair("crumb", crumb);
        }
    }
    let api_body = client.http().get(url).send().await?.text().await?;
    anyhow::ensure!(!api_body.trim().is_empty(), "empty API body");
    write_fixture(&format!("api/quoteSummary_{sym}.json"), &api_body);

    // 2) Quote HTML page (for scraper fallback)
    let mut url = client.base_quote().join(sym)?;
    { url.query_pairs_mut().append_pair("p", sym); }
    let html = client.http().get(url).send().await?.text().await?;
    anyhow::ensure!(html.contains("<html") || html.contains("QuoteSummaryStore"), "unexpected HTML");
    write_fixture(&format!("html/quote_{sym}.html"), &html);

    // 3) Chart (history)
    let mut url = client.base_chart().join(sym)?;
    {
        let mut qp = url.query_pairs_mut();
        qp.append_pair("range", "6mo");
        qp.append_pair("interval", "1d");
        qp.append_pair("events", "div|split");
    }
    let chart = client.http().get(url).send().await?.text().await?;
    anyhow::ensure!(chart.contains("chart"), "unexpected chart body");
    write_fixture(&format!("chart/{sym}_6mo.json"), &chart);

    Ok(())
}

#[tokio::test]
#[ignore] // opt-in: set YF_RECORD=1 to run
async fn record_live_fixtures() -> anyhow::Result<()> {
    if std::env::var("YF_RECORD").ok().as_deref() != Some("1") {
        return Ok(());
    }
    let mut client = YfClient::builder().build()?;

    // Choose the symbols you care about:
    for sym in ["AAPL", "QQQ"] {
        record_symbol(&mut client, sym).await?;
    }
    Ok(())
}
