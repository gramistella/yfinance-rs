mod common;

#[path = "ticker/actions.rs"]
mod actions;
#[path = "ticker/capital_gains.rs"]
mod capital_gains;
#[path = "ticker/components_live.rs"]
mod components_live;
#[path = "ticker/components_offline.rs"]
mod components_offline;
#[path = "ticker/data_quality.rs"]
mod data_quality;
#[path = "ticker/fast_info.rs"]
mod fast_info;
#[path = "ticker/history_convenience.rs"]
mod history_convenience;
#[path = "ticker/info_live.rs"]
mod info_live;
#[path = "ticker/info_offline.rs"]
mod info_offline;
#[path = "ticker/isin_live.rs"]
mod isin_live;
#[path = "ticker/isin_offline.rs"]
mod isin_offline;
#[path = "ticker/key_statistics.rs"]
mod key_statistics;
#[path = "ticker/live.rs"]
mod live;
#[path = "ticker/offline.rs"]
mod offline;
#[path = "ticker/options.rs"]
mod options;
#[path = "ticker/options_expiry_from_url_fallback.rs"]
mod options_expiry_from_url_fallback;
#[path = "ticker/profile.rs"]
mod profile;
#[path = "ticker/quote.rs"]
mod quote;
#[path = "ticker/shares.rs"]
mod shares;
