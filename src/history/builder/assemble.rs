use crate::core::models::Candle;
use crate::history::wire::QuoteBlock;

use super::adjust::price_factor_for_row;

pub(crate) fn assemble_candles(
    ts: &[i64],
    q: &QuoteBlock,
    adj: &[Option<f64>],
    auto_adjust: bool,
    keepna: bool,
    cum_split_after: &[f64],
) -> (Vec<Candle>, Vec<f64>) {
    let mut out = Vec::new();
    let mut raw_close_vec = Vec::new();

    for (i, &t) in ts.iter().enumerate() {
        let getter_f64 = |v: &Vec<Option<f64>>| v.get(i).and_then(|x| *x);
        let mut open = getter_f64(&q.open);
        let mut high = getter_f64(&q.high);
        let mut low = getter_f64(&q.low);
        let mut close = getter_f64(&q.close);
        let volume0 = q.volume.get(i).and_then(|x| *x);

        let raw_close_val = close.unwrap_or(f64::NAN);

        if auto_adjust {
            let pf = price_factor_for_row(i, adj.get(i).and_then(|x| *x), close, cum_split_after);

            if let Some(v) = open.as_mut() {
                *v *= pf;
            }
            if let Some(v) = high.as_mut() {
                *v *= pf;
            }
            if let Some(v) = low.as_mut() {
                *v *= pf;
            }
            if let Some(v) = close.as_mut() {
                *v *= pf;
            }

            let volume_adj = volume0.map(|v| {
                let v_adj = (v as f64) * cum_split_after[i];
                if v_adj.is_finite() {
                    v_adj.round() as u64
                } else {
                    v
                }
            });

            if let (Some(ov), Some(hv), Some(lv), Some(cv)) = (open, high, low, close) {
                out.push(Candle {
                    ts: t,
                    open: ov,
                    high: hv,
                    low: lv,
                    close: cv,
                    volume: volume_adj,
                });
                raw_close_vec.push(raw_close_val);
            } else if keepna {
                out.push(Candle {
                    ts: t,
                    open: open.unwrap_or(f64::NAN),
                    high: high.unwrap_or(f64::NAN),
                    low: low.unwrap_or(f64::NAN),
                    close: close.unwrap_or(f64::NAN),
                    volume: volume0,
                });
                raw_close_vec.push(raw_close_val);
            }
        } else if let (Some(ov), Some(hv), Some(lv), Some(cv)) = (open, high, low, close) {
            out.push(Candle {
                ts: t,
                open: ov,
                high: hv,
                low: lv,
                close: cv,
                volume: volume0,
            });
            raw_close_vec.push(raw_close_val);
        } else if keepna {
            out.push(Candle {
                ts: t,
                open: open.unwrap_or(f64::NAN),
                high: high.unwrap_or(f64::NAN),
                low: low.unwrap_or(f64::NAN),
                close: close.unwrap_or(f64::NAN),
                volume: volume0,
            });
            raw_close_vec.push(raw_close_val);
        }
    }

    (out, raw_close_vec)
}
