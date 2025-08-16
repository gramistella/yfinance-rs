// src/core/mod.rs
pub mod client;
pub mod error;
pub mod models;
pub mod services;

#[cfg(feature = "test-mode")]
pub(crate) mod fixtures;

pub(crate) mod net;

// convenient re-exports so most code can just `use crate::core::YfClient`
pub use client::YfClient;
pub use error::YfError;
pub use models::{Action, Candle, HistoryMeta, HistoryResponse, Interval, Quote, Range};
pub use services::{HistoryRequest, HistoryService};
