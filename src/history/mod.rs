mod builder;
mod model;
mod params;
mod wire; // internal-only serde mapping

pub use builder::HistoryBuilder;
pub use model::{Action, Candle, HistoryMeta, HistoryResponse};
pub use params::{Interval, Range};
