pub fn cumulative_split_after(ts: &[i64], split_events: &[(i64, f64)]) -> Vec<f64> {
    let mut out = vec![1.0; ts.len()];
    if split_events.is_empty() || ts.is_empty() {
        return out;
    }

    let mut sp_idx = split_events.len();
    let mut running: f64 = 1.0;

    for i in (0..ts.len()).rev() {
        while sp_idx > 0 && split_events[sp_idx - 1].0 > ts[i] {
            sp_idx -= 1;
            running *= split_events[sp_idx].1;
        }
        out[i] = running;
    }
    out
}

pub fn price_factor_for_row(
    i: usize,
    adjclose_i: Option<f64>,
    close_i: Option<f64>,
    cum_split_after: &[f64],
) -> f64 {
    match (adjclose_i, close_i) {
        (Some(adj), Some(c)) if c != 0.0 => adj / c,
        _ => 1.0 / cum_split_after[i].max(1e-12),
    }
}
