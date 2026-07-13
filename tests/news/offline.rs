use httpmock::{Method::POST, MockServer};
use serde_json::json;
use std::time::Duration;
use url::Url;
use yfinance_rs::{CacheMode, NewsTab, ProjectionIssue, Ticker, YfClient, YfError, YfWarning};

fn fixture(endpoint: &str, symbol: &str) -> String {
    crate::common::fixture(endpoint, symbol, "json")
}

const MALFORMED_NEWS_STREAM_BODY: &str = r#"{
  "data": {
    "tickerStream": {
      "stream": [
        {
          "id": 42,
          "content": {
            "title": "Bad id",
            "pubDate": "2025-01-01T00:00:00Z"
          }
        },
        {
          "content": {
            "title": "Missing id",
            "pubDate": "2025-01-01T00:00:00Z"
          }
        },
        {
          "id": "valid-news",
          "content": {
            "title": "Valid headline",
            "pubDate": "2025-01-01T00:00:00Z",
            "provider": {
              "displayName": 42
            },
            "canonicalUrl": {
              "url": 42
            }
          }
        }
      ]
    }
  }
}"#;

const SYNTHETIC_SUBSECOND_NEWS_STREAM_BODY: &str = r#"{
  "data": {
    "tickerStream": {
      "stream": [{
        "id": "subsecond-news",
        "content": {
          "title": "Precise publication time",
          "pubDate": "2025-01-01T00:00:00.123456789Z"
        }
      }]
    }
  }
}"#;

#[tokio::test]
async fn offline_news_uses_recorded_fixture() {
    let server = MockServer::start();
    let sym = "AAPL";

    let expected_payload = json!({
        "serviceConfig": {
            "snippetCount": 10,
            "s": [sym]
        }
    });

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "latestNews")
            .query_param("serviceKey", "ncp_fin")
            .json_body(expected_payload);
        then.status(200)
            .header("content-type", "application/json")
            // Use the new, specific fixture name
            .body(fixture("news_latestNews", sym));
    });

    let client = YfClient::builder()
        .base_news(Url::parse(&server.base_url()).unwrap())
        .build()
        .unwrap();

    let ticker = Ticker::new(&client, sym);
    let articles = ticker.news().await.unwrap();

    mock.assert();

    // Make the assertion flexible: check that we got some articles, not an exact number.
    assert!(
        !articles.is_empty(),
        "Expected to parse at least one article from the fixture"
    );

    // Perform general checks on the first article
    let first = &articles[0];
    assert!(!first.uuid.is_empty());
    assert!(!first.title.is_empty());
    assert!(first.published_at.timestamp() > 0);
}

#[tokio::test]
async fn synthetic_news_preserves_rfc3339_subsecond_precision() {
    let server = MockServer::start();
    let sym = "AAPL";

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "latestNews")
            .query_param("serviceKey", "ncp_fin");
        then.status(200)
            .header("content-type", "application/json")
            .body(SYNTHETIC_SUBSECOND_NEWS_STREAM_BODY);
    });

    let client = YfClient::builder()
        .base_news(Url::parse(&server.base_url()).unwrap())
        .build()
        .unwrap();

    let articles = Ticker::new(&client, sym).news().await.unwrap();

    mock.assert();
    assert_eq!(articles.len(), 1);
    assert_eq!(
        articles[0].published_at.timestamp_subsec_nanos(),
        123_456_789
    );
}

#[tokio::test]
async fn offline_news_builder_configures_request() {
    let server = MockServer::start();
    let sym = "AAPL";

    let expected_payload = json!({
        "serviceConfig": {
            "snippetCount": 5,
            "s": [sym]
        }
    });

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "pressRelease") // Corresponds to NewsTab::PressReleases
            .query_param("serviceKey", "ncp_fin")
            .json_body(expected_payload);
        then.status(200)
            .header("content-type", "application/json")
            // Use the new, specific fixture for press releases
            .body(fixture("news_pressRelease", sym));
    });

    let client = YfClient::builder()
        .base_news(Url::parse(&server.base_url()).unwrap())
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, sym);

    let _articles = ticker
        .news_builder()
        .count(5)
        .tab(NewsTab::PressReleases)
        .fetch()
        .await
        .unwrap();

    mock.assert();
}

#[tokio::test]
async fn default_news_cache_mode_bypasses_response_cache() {
    let server = MockServer::start();
    let sym = "AAPL";
    let expected_payload = json!({
        "serviceConfig": {
            "snippetCount": 10,
            "s": [sym]
        }
    });

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "latestNews")
            .query_param("serviceKey", "ncp_fin")
            .json_body(expected_payload);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("news_latestNews", sym));
    });

    let client = YfClient::builder()
        .base_news(Url::parse(&server.base_url()).unwrap())
        .cache_ttl(Duration::from_mins(1))
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, sym);

    assert!(!ticker.news().await.unwrap().is_empty());
    assert!(!ticker.news().await.unwrap().is_empty());
    mock.assert_calls(2);
}

#[tokio::test]
async fn explicit_news_cache_mode_uses_body_aware_response_cache() {
    let server = MockServer::start();
    let sym = "AAPL";
    let latest_payload = json!({
        "serviceConfig": {
            "snippetCount": 10,
            "s": [sym]
        }
    });
    let shorter_payload = json!({
        "serviceConfig": {
            "snippetCount": 5,
            "s": [sym]
        }
    });

    let latest = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "latestNews")
            .query_param("serviceKey", "ncp_fin")
            .json_body(latest_payload);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("news_latestNews", sym));
    });
    let shorter = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "latestNews")
            .query_param("serviceKey", "ncp_fin")
            .json_body(shorter_payload);
        then.status(200)
            .header("content-type", "application/json")
            .body(fixture("news_latestNews", sym));
    });

    let client = YfClient::builder()
        .base_news(Url::parse(&server.base_url()).unwrap())
        .cache_ttl(Duration::from_mins(1))
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, sym);

    assert!(
        !ticker
            .news_builder()
            .cache_mode(CacheMode::Use)
            .fetch()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !ticker
            .news_builder()
            .count(5)
            .cache_mode(CacheMode::Use)
            .fetch()
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        !ticker
            .news_builder()
            .cache_mode(CacheMode::Use)
            .fetch()
            .await
            .unwrap()
            .is_empty()
    );

    latest.assert_calls(1);
    shorter.assert_calls(1);
}

#[tokio::test]
async fn malformed_news_stream_items_are_dropped_with_diagnostics() {
    let server = MockServer::start();
    let sym = "AAPL";
    let expected_payload = json!({
        "serviceConfig": {
            "snippetCount": 10,
            "s": [sym]
        }
    });

    let mock = server.mock(|when, then| {
        when.method(POST)
            .path("/xhr/ncp")
            .query_param("queryRef", "latestNews")
            .query_param("serviceKey", "ncp_fin")
            .json_body(expected_payload);
        then.status(200)
            .header("content-type", "application/json")
            .body(MALFORMED_NEWS_STREAM_BODY);
    });

    let client = YfClient::builder()
        .base_news(Url::parse(&server.base_url()).unwrap())
        .build()
        .unwrap();
    let ticker = Ticker::new(&client, sym);

    let response = ticker
        .news_builder()
        .fetch_with_diagnostics()
        .await
        .unwrap();

    assert_eq!(response.data.len(), 1);
    assert_eq!(response.data[0].uuid, "valid-news");
    assert_eq!(response.data[0].title, "Valid headline");
    assert!(response.data[0].publisher.is_none());
    assert!(response.data[0].link.is_none());
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::DroppedItem {
            endpoint: "news",
            item: "news_article",
            key: Some(key),
            reason: ProjectionIssue::InvalidField {
                field: "id",
                ..
            },
        } if key == "stream[0]"
    )));
    assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
        warning,
        YfWarning::DroppedItem {
            endpoint: "news",
            item: "news_article",
            key: Some(key),
            reason: ProjectionIssue::MissingRequiredField { field: "id" },
        } if key == "stream[1]"
    )));
    for (path, field) in [
        ("provider.displayName", "displayName"),
        ("canonicalUrl.url", "url"),
    ] {
        assert!(response.diagnostics.warnings.iter().any(|warning| matches!(
            warning,
            YfWarning::OmittedPresentField {
                endpoint: "news",
                path: warning_path,
                key: Some(key),
                reason: ProjectionIssue::InvalidField {
                    field: warning_field,
                    ..
                },
            } if *warning_path == path && *warning_field == field && key == "valid-news"
        )));
    }

    let err = ticker.news_builder().strict().fetch().await.unwrap_err();

    mock.assert_calls(2);
    assert!(matches!(err, YfError::DataQuality(_)));
}
