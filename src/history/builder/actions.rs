use crate::core::models::Action;
use crate::history::wire::Events;

pub(crate) fn extract_actions(events: &Option<Events>) -> (Vec<Action>, Vec<(i64, f64)>) {
    let mut out: Vec<Action> = Vec::new();
    let mut split_events: Vec<(i64, f64)> = Vec::new();

    let Some(ev) = events.as_ref() else {
        return (out, split_events);
    };

    if let Some(divs) = ev.dividends.as_ref() {
        for (k, d) in divs {
            let ts = k.parse::<i64>().unwrap_or(d.date.unwrap_or(0));
            if let Some(amount) = d.amount {
                out.push(Action::Dividend { ts, amount });
            }
        }
    }

    if let Some(gains) = ev.capital_gains.as_ref() {
        for (k, g) in gains {
            let ts = k.parse::<i64>().unwrap_or(g.date.unwrap_or(0));
            if let Some(gain) = g.amount {
                out.push(Action::CapitalGain { ts, gain });
            }
        }
    }

    if let Some(splits) = ev.splits.as_ref() {
        for (k, s) in splits {
            let ts = k.parse::<i64>().unwrap_or(s.date.unwrap_or(0));
            let (num, den) = if let (Some(n), Some(d)) = (s.numerator, s.denominator) {
                (n as u32, d as u32)
            } else if let Some(r) = s.split_ratio.as_deref() {
                let mut it = r.split('/');
                let n = it.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(1);
                let d = it.next().and_then(|x| x.parse::<u32>().ok()).unwrap_or(1);
                (n, d)
            } else {
                (1, 1)
            };

            out.push(Action::Split {
                ts,
                numerator: num,
                denominator: den,
            });

            let ratio = if den == 0 {
                1.0
            } else {
                (num as f64) / (den as f64)
            };
            split_events.push((ts, ratio));
        }
    }

    out.sort_by_key(|a| match *a {
        Action::Dividend { ts, .. } | Action::Split { ts, .. } | Action::CapitalGain { ts, .. } => {
            ts
        }
    });
    split_events.sort_by_key(|(ts, _)| *ts);

    (out, split_events)
}
