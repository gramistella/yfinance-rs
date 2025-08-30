//! Core components of the `yfinance-rs` client.
//!
//! This module contains the foundational building blocks of the library, including:
//! - The main [`YfClient`] and its builder.
//! - The primary [`YfError`] type.
//! - Shared data models like [`Quote`] and [`Candle`].
//! - Internal networking and authentication logic.

/// The main client (`YfClient`), builder, and configuration.
pub mod client;
/// The primary error type (`YfError`) for the crate.
pub mod error;
/// Shared data models used across multiple API modules (e.g., `Quote`, `Candle`).
pub mod models;
pub(crate) mod quotes;
pub(crate) mod quotesummary;
/// Service traits for abstracting functionality like history fetching.
pub mod services;
pub(crate) mod wire;

#[cfg(feature = "test-mode")]
pub(crate) mod fixtures;

pub(crate) mod net;

// convenient re-exports so most code can just `use crate::core::YfClient`
pub use client::{YfClient, YfClientBuilder};
pub use error::YfError;
pub use models::{Action, Candle, HistoryMeta, HistoryResponse, Interval, Quote, Range};
pub use services::{HistoryRequest, HistoryService};
