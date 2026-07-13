mod common;

#[path = "history/adjust_from_splits_only.rs"]
mod adjust_from_splits_only;
#[path = "history/action_currency_synthetic.rs"]
mod history_action_currency_synthetic;
#[path = "history/adjust.rs"]
mod history_adjust;
#[path = "history/fractional_splits.rs"]
mod history_fractional_splits;
#[path = "history/intervals.rs"]
mod history_intervals;
#[path = "history/live.rs"]
mod history_live;
#[path = "history/meta.rs"]
mod history_meta;
#[path = "history/nulls_synthetic.rs"]
mod history_nulls_synth;
#[path = "history/offline.rs"]
mod history_offline;
#[path = "history/params.rs"]
mod history_params;
#[path = "history/ranges_new.rs"]
mod history_ranges_new;
#[path = "history/smoke.rs"]
mod history_smoke;
#[path = "history/volume_adjustment.rs"]
mod history_volume_adjustment;

#[path = "history/malformed_ohlc.rs"]
mod malformed_ohlc;

#[path = "history/http_status_error.rs"]
mod http_status_error;

#[path = "history/retry_synthetic.rs"]
mod retry_synthetic;

#[path = "history/caching_synthetic.rs"]
mod caching_synthetic;

#[path = "history/decimal_precision_synthetic.rs"]
mod decimal_precision_synthetic;
