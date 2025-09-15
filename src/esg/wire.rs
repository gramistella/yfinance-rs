use crate::core::wire::RawNum;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct V10Result {
    #[serde(rename = "esgScores")]
    pub(crate) esg_scores: Option<EsgScoresNode>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EsgScoresNode {
    // These are objects: { "raw": ... }
    #[allow(dead_code)]
    pub(crate) total_esg: Option<RawNum<f64>>,
    pub(crate) environment_score: Option<RawNum<f64>>,
    pub(crate) social_score: Option<RawNum<f64>>,
    pub(crate) governance_score: Option<RawNum<f64>>,

    // These are primitives
    #[allow(dead_code)]
    pub(crate) percentile: Option<f64>,
    #[allow(dead_code)]
    pub(crate) highest_controversy: Option<f64>, // Use f64 to match JSON `2.0`

    // Involvement flags
    pub(crate) adult: Option<bool>,
    pub(crate) alcoholic: Option<bool>,
    pub(crate) animal_testing: Option<bool>,
    pub(crate) catholic: Option<bool>,
    pub(crate) controversial_weapons: Option<bool>,
    pub(crate) small_arms: Option<bool>,
    pub(crate) fur_leather: Option<bool>,
    pub(crate) gambling: Option<bool>,
    pub(crate) gmo: Option<bool>,
    pub(crate) military_contract: Option<bool>,
    pub(crate) nuclear: Option<bool>,
    pub(crate) palm_oil: Option<bool>,
    pub(crate) pesticides: Option<bool>,
    #[serde(rename = "coal")] // JSON key is "coal", map to our more descriptive field name
    pub(crate) thermal_coal: Option<bool>,
    pub(crate) tobacco: Option<bool>,
}
