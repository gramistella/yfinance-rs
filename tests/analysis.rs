// tests/analysis.rs
mod common;

#[path = "analysis/live.rs"]
mod analysis_live;
#[path = "analysis/offline.rs"]
mod analysis_offline;
#[path = "analysis/retry_synthetic.rs"]
mod analysis_retry_synth;
#[path = "analysis/earnings_trend.rs"]
mod earnings_trend;
#[path = "analysis/price_target.rs"]
mod price_target;
#[path = "analysis/price_target_live.rs"]
mod price_target_live;
#[path = "analysis/sorted_upgrades.rs"]
mod sorted_upgrades;
#[path = "analysis/yahoo_error_passthrough.rs"]
mod yahoo_error_passthrough;
