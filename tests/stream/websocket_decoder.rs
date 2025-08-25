use super::common;

#[test]
fn decode_real_websocket_message() {
    let base64_msg = common::fixture("stream_ws", "MULTI", "b64");
    let update = yfinance_rs::stream::decode_and_map_message(&base64_msg).unwrap();

    // Generic assertions, as the symbol/price will change with each recording
    assert!(!update.symbol.is_empty(), "symbol should not be empty");
    assert!(update.last_price.is_some(), "price should be present");
    assert!(
        update.last_price.unwrap() > 0.0,
        "price should be positive"
    );
    assert!(update.ts > 1_000_000_000, "timestamp should be valid");
}