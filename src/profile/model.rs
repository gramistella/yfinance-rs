use serde::Serialize;

/// Represents a physical address for a company.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Address {
    /// Street address line 1.
    pub street1: Option<String>,
    /// Street address line 2.
    pub street2: Option<String>,
    /// City.
    pub city: Option<String>,
    /// State or province.
    pub state: Option<String>,
    /// Country.
    pub country: Option<String>,
    /// Postal code.
    pub zip: Option<String>,
}

/// Represents the profile of a publicly traded company.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Company {
    /// The full name of the company.
    pub name: String,
    /// The business sector the company operates in.
    pub sector: Option<String>,
    /// The specific industry within the sector.
    pub industry: Option<String>,
    /// The company's official website.
    pub website: Option<String>,
    /// The physical address of the company's headquarters.
    pub address: Option<Address>,
    /// A summary of the company's business operations.
    pub summary: Option<String>,
    /// The International Securities Identification Number.
    pub isin: Option<String>,
}

/// Represents the profile of a fund (e.g., an ETF).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fund {
    /// The full name of the fund.
    pub name: String,
    /// The family of funds it belongs to (e.g., "iShares").
    pub family: Option<String>,
    /// The legal type of the fund (e.g., "Exchange Traded Fund").
    pub kind: String,
    /// The International Securities Identification Number.
    pub isin: Option<String>,
}

/// An enum representing either a `Company` or a `Fund` profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Profile {
    /// The profile for a company stock.
    Company(Company),
    /// The profile for a fund.
    Fund(Fund),
}

impl Profile {
    /// Returns the ISIN for the company or fund, if available.
    #[must_use]
    pub fn isin(&self) -> Option<&str> {
        match self {
            Self::Company(c) => c.isin.as_deref(),
            Self::Fund(f) => f.isin.as_deref(),
        }
    }
}
