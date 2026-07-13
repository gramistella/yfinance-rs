use std::fmt;

use super::{ProjectionIssue, YfCurrencyInference, YfCurrencyPurpose};

/// A warning emitted by the Yahoo projection layer.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum YfWarning {
    /// A provider item was skipped because it could not form a valid public value.
    DroppedItem {
        /// Stable endpoint or mapper name.
        endpoint: &'static str,
        /// Stable item kind, such as `candle`, `dividend`, or `news_article`.
        item: &'static str,
        /// Provider key, timestamp, symbol, or row identifier when available.
        key: Option<String>,
        /// Why the item was dropped.
        reason: ProjectionIssue,
    },
    /// A provider field was present but omitted from an otherwise valid public item.
    OmittedPresentField {
        /// Stable endpoint or mapper name.
        endpoint: &'static str,
        /// Stable field path within the provider payload.
        path: &'static str,
        /// Provider key, timestamp, symbol, or row identifier when available.
        key: Option<String>,
        /// Why the field was omitted.
        reason: ProjectionIssue,
    },
    /// A requested provider module or feature was unavailable.
    ProviderFeatureUnavailable {
        /// Stable endpoint or mapper name.
        endpoint: &'static str,
        /// Feature or module name.
        feature: &'static str,
        /// Why the feature was unavailable.
        reason: ProjectionIssue,
    },
    /// A provider value was repaired before being returned.
    RepairedData {
        /// Stable endpoint or mapper name.
        endpoint: &'static str,
        /// Stable item kind.
        item: &'static str,
        /// Provider key, timestamp, symbol, or row identifier when available.
        key: Option<String>,
        /// Description of the repair.
        repair: &'static str,
    },
    /// Currency was inferred from a heuristic because Yahoo did not provide a usable currency field.
    CurrencyInferred {
        /// Stable endpoint or mapper name.
        endpoint: &'static str,
        /// Symbol whose currency was resolved.
        symbol: String,
        /// Currency purpose.
        purpose: YfCurrencyPurpose,
        /// Heuristic used to infer the currency.
        inference: YfCurrencyInference,
    },
    /// An aggregate endpoint suppressed a subcall error and returned partial data.
    SuppressedError {
        /// Stable endpoint or mapper name.
        endpoint: &'static str,
        /// Subcall or operation that failed.
        operation: &'static str,
        /// Error text.
        error: String,
    },
}

impl fmt::Display for YfWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DroppedItem {
                endpoint,
                item,
                key,
                reason,
            } => write!(
                f,
                "{endpoint}: dropped {item}{}: {reason}",
                fmt_key(key.as_deref())
            ),
            Self::OmittedPresentField {
                endpoint,
                path,
                key,
                reason,
            } => write!(
                f,
                "{endpoint}: omitted present field {path}{}: {reason}",
                fmt_key(key.as_deref())
            ),
            Self::ProviderFeatureUnavailable {
                endpoint,
                feature,
                reason,
            } => write!(
                f,
                "{endpoint}: provider feature {feature} unavailable: {reason}"
            ),
            Self::RepairedData {
                endpoint,
                item,
                key,
                repair,
            } => write!(
                f,
                "{endpoint}: repaired {item}{}: {repair}",
                fmt_key(key.as_deref())
            ),
            Self::CurrencyInferred {
                endpoint,
                symbol,
                purpose,
                inference,
            } => write!(
                f,
                "{endpoint}: inferred {purpose} currency for {symbol} from {inference} (no usable provider currency)"
            ),
            Self::SuppressedError {
                endpoint,
                operation,
                error,
            } => write!(f, "{endpoint}: suppressed {operation} error: {error}"),
        }
    }
}

impl YfWarning {
    pub(crate) fn with_key_prefix(self, prefix: &str) -> Self {
        match self {
            Self::DroppedItem {
                endpoint,
                item,
                key,
                reason,
            } => Self::DroppedItem {
                endpoint,
                item,
                key: Some(prefixed_key(prefix, key)),
                reason,
            },
            Self::OmittedPresentField {
                endpoint,
                path,
                key,
                reason,
            } => Self::OmittedPresentField {
                endpoint,
                path,
                key: Some(prefixed_key(prefix, key)),
                reason,
            },
            Self::RepairedData {
                endpoint,
                item,
                key,
                repair,
            } => Self::RepairedData {
                endpoint,
                item,
                key: Some(prefixed_key(prefix, key)),
                repair,
            },
            other => other,
        }
    }
}

fn fmt_key(key: Option<&str>) -> String {
    key.map_or_else(String::new, |key| format!(" [{key}]"))
}

fn prefixed_key(prefix: &str, key: Option<String>) -> String {
    key.map_or_else(|| prefix.to_string(), |key| format!("{prefix}:{key}"))
}
