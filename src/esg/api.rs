use crate::{
    core::{
        YfClient, YfError,
        client::{CacheMode, RetryConfig},
        quotesummary,
        wire::from_raw,
    },
    esg::{
        model::{EsgInvolvement, EsgScores},
        wire::V10Result,
    },
};

pub(super) async fn fetch_esg_scores(
    client: &YfClient,
    symbol: &str,
    cache_mode: CacheMode,
    retry_override: Option<&RetryConfig>,
) -> Result<EsgScores, YfError> {
    let root: V10Result = quotesummary::fetch_module_result(
        client,
        symbol,
        "esgScores",
        "esg",
        cache_mode,
        retry_override,
    )
    .await?;

    let esg = root
        .esg_scores
        .ok_or_else(|| YfError::Data("esgScores module missing from response".into()))?;

    let b = |x: Option<bool>| x.unwrap_or(false);

    Ok(EsgScores {
        total_esg: from_raw(esg.total_esg),
        environment_score: from_raw(esg.environment_score),
        social_score: from_raw(esg.social_score),
        governance_score: from_raw(esg.governance_score),
        esg_percentile: esg.percentile,
        highest_controversy: esg.highest_controversy.and_then(|v| {
            let rounded = v.round();
            if rounded >= 0.0 && rounded <= f64::from(u32::MAX) {
                // This cast is safe as we check the bounds of rounded.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                Some(rounded as u32)
            } else {
                None
            }
        }),
        involvement: EsgInvolvement {
            adult: b(esg.adult),
            alcoholic: b(esg.alcoholic),
            animal_testing: b(esg.animal_testing),
            catholic: b(esg.catholic),
            controversial_weapons: b(esg.controversial_weapons),
            small_arms: b(esg.small_arms),
            fur_leather: b(esg.fur_leather),
            gambling: b(esg.gambling),
            gmo: b(esg.gmo),
            military_contract: b(esg.military_contract),
            nuclear: b(esg.nuclear),
            palm_oil: b(esg.palm_oil),
            pesticides: b(esg.pesticides),
            thermal_coal: b(esg.thermal_coal),
            tobacco: b(esg.tobacco),
        },
    })
}
