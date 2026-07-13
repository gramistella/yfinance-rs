use crate::core::{
    ProjectionIssue, YfDiagnostics, YfError, YfResponse, YfWarning,
    currency_resolver::{CurrencyPurpose, ResolvedCurrency},
    diagnostics::{DataQuality, YfCurrencyInference, YfCurrencyPurpose},
};

#[derive(Debug, Clone)]
pub struct ProjectionContext {
    endpoint: &'static str,
    policy: DataQuality,
    diagnostics: YfDiagnostics,
}

impl ProjectionContext {
    pub(crate) const fn new(endpoint: &'static str, policy: DataQuality) -> Self {
        Self {
            endpoint,
            policy,
            diagnostics: YfDiagnostics::new(),
        }
    }

    pub(crate) const fn policy(&self) -> DataQuality {
        self.policy
    }

    pub(crate) fn record(&mut self, warning: YfWarning) -> Result<(), YfError> {
        match self.policy {
            DataQuality::BestEffort => {
                self.diagnostics.push(warning);
                Ok(())
            }
            DataQuality::Strict => Err(YfError::DataQuality(Box::new(warning))),
        }
    }

    pub(crate) fn dropped_item(
        &mut self,
        item: &'static str,
        key: Option<&str>,
        reason: ProjectionIssue,
    ) -> Result<(), YfError> {
        self.record(YfWarning::DroppedItem {
            endpoint: self.endpoint,
            item,
            key: key.map(str::to_owned),
            reason,
        })
    }

    pub(crate) fn omitted_present_field(
        &mut self,
        path: &'static str,
        key: Option<&str>,
        reason: ProjectionIssue,
    ) -> Result<(), YfError> {
        self.record(YfWarning::OmittedPresentField {
            endpoint: self.endpoint,
            path,
            key: key.map(str::to_owned),
            reason,
        })
    }

    pub(crate) fn suppressed_error(
        &mut self,
        operation: &'static str,
        error: &YfError,
    ) -> Result<(), YfError> {
        self.record(YfWarning::SuppressedError {
            endpoint: self.endpoint,
            operation,
            error: error.to_string(),
        })
    }

    pub(crate) fn provider_feature_unavailable(
        &mut self,
        feature: &'static str,
        reason: ProjectionIssue,
    ) -> Result<(), YfError> {
        self.record(YfWarning::ProviderFeatureUnavailable {
            endpoint: self.endpoint,
            feature,
            reason,
        })
    }

    pub(crate) fn unavailable_feature(&mut self, feature: &'static str) -> Result<(), YfError> {
        self.provider_feature_unavailable(feature, ProjectionIssue::ProviderUnavailable { feature })
    }

    pub(crate) fn repaired_data(
        &mut self,
        item: &'static str,
        key: Option<&str>,
        repair: &'static str,
    ) -> Result<(), YfError> {
        self.record(YfWarning::RepairedData {
            endpoint: self.endpoint,
            item,
            key: key.map(str::to_owned),
            repair,
        })
    }

    pub(crate) fn currency_resolution(
        &mut self,
        symbol: &str,
        purpose: CurrencyPurpose,
        resolved: &ResolvedCurrency,
    ) -> Result<(), YfError> {
        match resolved.evidence().inference() {
            Some(inference) => self.record(YfWarning::CurrencyInferred {
                endpoint: self.endpoint,
                symbol: symbol.to_string(),
                purpose: YfCurrencyPurpose::from(purpose),
                inference: YfCurrencyInference::from(inference),
            }),
            None => Ok(()),
        }
    }

    pub(crate) fn extend(&mut self, diagnostics: YfDiagnostics) {
        self.diagnostics.extend(diagnostics);
    }

    pub(crate) fn finish<T>(self, data: T) -> YfResponse<T> {
        YfResponse::new(data, self.diagnostics)
    }
}
