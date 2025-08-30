use serde::Serialize;

/// A container for all ESG (Environmental, Social, and Governance) scores for a company.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EsgScores {
    /// The total ESG score, which is a weighted average of the three component scores.
    pub total_esg: Option<f64>,
    /// The environmental score, measuring the company's impact on the environment.
    pub environment_score: Option<f64>,
    /// The social score, measuring the company's performance on social issues like labor practices and human rights.
    pub social_score: Option<f64>,
    /// The governance score, measuring the company's corporate governance practices.
    pub governance_score: Option<f64>,
    /// The company's ESG score percentile rank compared to its peers.
    pub esg_percentile: Option<f64>,
    /// The highest level of controversy the company has been involved in.
    pub highest_controversy: Option<u32>,
    /// Flags indicating the company's involvement in various controversial sectors.
    pub involvement: EsgInvolvement,
}

/// Flags indicating a company's involvement in specific controversial business sectors.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
#[allow(clippy::struct_excessive_bools)]
pub struct EsgInvolvement {
    /// Involvement in the adult entertainment industry.
    pub adult: bool,
    /// Involvement in the production or sale of alcoholic beverages.
    pub alcoholic: bool,
    /// Involvement in animal testing for non-medical purposes.
    pub animal_testing: bool,
    /// Adherence to Catholic principles in business practices.
    pub catholic: bool,
    /// Involvement in the production of controversial weapons.
    pub controversial_weapons: bool,
    /// Involvement in the production or sale of small arms.
    pub small_arms: bool,
    /// Involvement in the fur and leather industry.
    pub fur_leather: bool,
    /// Involvement in the gambling industry.
    pub gambling: bool,
    /// Involvement in genetically modified organisms (GMOs).
    pub gmo: bool,
    /// Involvement as a military contractor.
    pub military_contract: bool,
    /// Involvement in the nuclear power industry.
    pub nuclear: bool,
    /// Involvement in the palm oil industry.
    pub palm_oil: bool,
    /// Involvement in the production of pesticides.
    pub pesticides: bool,
    /// Involvement in the thermal coal industry.
    pub thermal_coal: bool,
    /// Involvement in the tobacco industry.
    pub tobacco: bool,
}
