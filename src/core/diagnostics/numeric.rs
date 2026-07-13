use crate::core::{ProjectionContext, ProjectionIssue, YfError, diagnostics::optional_projected};

pub fn optional_u32_from_i64(
    ctx: &mut ProjectionContext,
    path: &'static str,
    key: Option<&str>,
    field: &'static str,
    value: Option<i64>,
) -> Result<Option<u32>, YfError> {
    optional_projected(ctx, path, key, value, |value| {
        u32::try_from(value).map_err(|_| invalid_u32_count(field, value))
    })
}

fn invalid_u32_count(field: &'static str, value: impl std::fmt::Display) -> ProjectionIssue {
    ProjectionIssue::InvalidField {
        field,
        details: format!(
            "expected finite integer count in 0..={}, got {value}",
            u32::MAX
        ),
    }
}
