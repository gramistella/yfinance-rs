use super::common;
use paft::domain::IdentifierScheme;

#[test]
fn decode_real_websocket_message() {
    let base64_msg = common::fixture("stream_ws", "MULTI", "b64");
    let update = yfinance_rs::stream::decode_and_map_message(&base64_msg).unwrap();

    // Generic assertions, as the symbol/price will change with each recording
    match update.instrument.id() {
        IdentifierScheme::Security(s) => {
            assert!(!s.symbol.is_empty(), "symbol should not be empty");
        }
        IdentifierScheme::Prediction(_) => panic!("unexpected instrument identifier scheme"),
    }
    assert!(update.price.is_some(), "price should be present");
    assert!(
        update
            .price
            .as_ref()
            .map_or(0.0, yfinance_rs::core::conversions::money_to_f64)
            > 0.0,
        "price should be positive"
    );

    // Decoder is stateless: volume must be None
    assert!(update.volume.is_none(), "decoder should not set volume");
}
