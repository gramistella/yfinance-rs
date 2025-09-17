use std::collections::HashSet;

use httpmock::{Method::GET, MockServer};
use serde_json::Value;
use url::Url;
use yfinance_rs::{Ticker, YfClient};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

fn expected_involvement_from_fixture(sym: &str) -> HashSet<String> {
    let body = fixture("esg_api_esgScores", sym);
    let v: Value = serde_json::from_str(&body).expect("valid JSON");
    let scores = &v["quoteSummary"]["result"][0]["esgScores"];

    // Map possible JSON keys to our category strings
    let candidates: &[(&[&str], &str)] = &[
        (&["adult"], "adult"),
        (&["alcoholic"], "alcoholic"),
        (&["animalTesting", "animal_testing"], "animal_testing"),
        (&["catholic"], "catholic"),
        (
            &["controversialWeapons", "controversial_weapons"],
            "controversial_weapons",
        ),
        (&["smallArms", "small_arms"], "small_arms"),
        (&["furLeather", "fur_leather"], "fur_leather"),
        (&["gambling"], "gambling"),
        (&["gmo"], "gmo"),
        (
            &["militaryContract", "military_contract"],
            "military_contract",
        ),
        (&["nuclear"], "nuclear"),
        (&["palmOil", "palm_oil"], "palm_oil"),
        (&["pesticides"], "pesticides"),
        (&["thermalCoal", "thermal_coal"], "thermal_coal"),
        (&["tobacco"], "tobacco"),
    ];

    let mut out = HashSet::new();
    for (keys, category) in candidates {
        let mut is_true = false;
        for key in *keys {
            if scores.get(*key).and_then(Value::as_bool).unwrap_or(false) {
                is_true = true;
                break;
            }
        }
        if is_true {
            out.insert((*category).to_string());
        }
    }
    out
}

#[tokio::test]
async fn offline_esg_involvement_matches_fixture() {
    let sym = "MSFT";
    let server = MockServer::start();

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path(format!("/v10/finance/quoteSummary/{sym}"))
            .query_param("modules", "esgScores")
            .query_param("crumb", "crumb");
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("esg_api_esgScores", sym));
    });

    let client = YfClient::builder()
        .base_quote_api(
            Url::parse(&format!("{}/v10/finance/quoteSummary/", server.base_url())).unwrap(),
        )
        ._preauth("cookie", "crumb")
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let summary = ticker.sustainability().await.unwrap();

    mock.assert();

    let got: HashSet<String> = summary
        .involvement
        .iter()
        .map(|i| i.category.clone())
        .collect();
    let expected = expected_involvement_from_fixture(sym);

    assert_eq!(
        got, expected,
        "involvement categories should match fixture booleans"
    );
}
