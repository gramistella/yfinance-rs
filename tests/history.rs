mod common;

#[path = "history/adjust_from_splits_only.rs"]
mod adjust_from_splits_only;
#[path = "history/adjust.rs"]
mod history_adjust;
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

#[path = "history/keepna_true.rs"]
mod keepna_true;

#[path = "history/http_status_error.rs"]
mod http_status_error;

#[path = "history/retry_synthetic.rs"]
mod retry_synthetic;

#[path = "history/caching_synthetic.rs"]
mod caching_synthetic;