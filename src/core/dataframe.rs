use polars::prelude::*;

/// Trait for converting financial data structures into Polars DataFrames.
///
/// This trait provides a consistent interface for converting various yfinance-rs data
/// structures into Polars DataFrames for advanced data analysis and manipulation.
pub trait ToDataFrame {
    /// Converts the object into a Polars DataFrame.
    fn to_dataframe(&self) -> PolarsResult<DataFrame>;

    /// Creates an empty DataFrame with the correct schema for this type.
    fn empty_dataframe() -> PolarsResult<DataFrame> where Self: Sized;

    /// Returns the complete flattened schema for this type.
    /// 
    /// This method provides static access to the type's schema without requiring
    /// an instance, making it useful for building nested schemas and validating
    /// data structures at compile time.
    fn schema() -> PolarsResult<Vec<(&'static str, DataType)>> where Self: Sized;
}
