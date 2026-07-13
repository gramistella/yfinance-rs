use crate::{
    core::{
        CallOptions, DataQuality, ProjectionContext, YfClient, YfError, YfResponse,
        diagnostics::WireProjection, quotesummary,
    },
    esg::wire::V10Result,
};
use paft::fundamentals::esg::{EsgInvolvement, EsgScores, EsgSummary};

pub(super) async fn fetch_esg_scores(
    client: &YfClient,
    symbol: &str,
    options: &CallOptions,
) -> Result<YfResponse<EsgSummary>, YfError> {
    let root: V10Result =
        quotesummary::fetch_module_result(client, symbol, "esgScores", "esg", options).await?;

    map_esg_scores(symbol, root, options.data_quality())
}

fn map_esg_scores(
    symbol: &str,
    root: V10Result,
    data_quality: DataQuality,
) -> Result<YfResponse<EsgSummary>, YfError> {
    let mut ctx = ProjectionContext::new("esg", data_quality);
    let Some(esg) = root.esg_scores else {
        ctx.unavailable_feature("esgScores")?;
        return Ok(ctx.finish(EsgSummary {
            scores: None,
            involvement: Vec::new(),
        }));
    };

    let environmental = esg.environment_score.optional_raw_field(
        &mut ctx,
        "esgScores.environmentScore",
        Some(symbol),
        "environmentScore",
    )?;
    let social = esg.social_score.optional_raw_field(
        &mut ctx,
        "esgScores.socialScore",
        Some(symbol),
        "socialScore",
    )?;
    let governance = esg.governance_score.optional_raw_field(
        &mut ctx,
        "esgScores.governanceScore",
        Some(symbol),
        "governanceScore",
    )?;

    // Map to paft types: paft::fundamentals::EsgScores now has only environmental/social/governance.
    let scores = EsgScores {
        environmental,
        social,
        governance,
    };

    // Collect involvement booleans as individual entries with simple categories.
    let mut involvement: Vec<EsgInvolvement> = Vec::new();
    let mut push_flag = |name: &str, val: Option<bool>| {
        let Some(val) = val else {
            return;
        };
        if val {
            involvement.push(EsgInvolvement {
                category: name.to_string(),
                score: None,
            });
        }
    };
    push_flag("adult", esg.adult);
    push_flag("alcoholic", esg.alcoholic);
    push_flag("animal_testing", esg.animal_testing);
    push_flag("catholic", esg.catholic);
    push_flag("controversial_weapons", esg.controversial_weapons);
    push_flag("small_arms", esg.small_arms);
    push_flag("fur_leather", esg.fur_leather);
    push_flag("gambling", esg.gambling);
    push_flag("gmo", esg.gmo);
    push_flag("military_contract", esg.military_contract);
    push_flag("nuclear", esg.nuclear);
    push_flag("palm_oil", esg.palm_oil);
    push_flag("pesticides", esg.pesticides);
    push_flag("thermal_coal", esg.thermal_coal);
    push_flag("tobacco", esg.tobacco);

    // Return scores together with involvement in a single summary
    Ok(ctx.finish(EsgSummary {
        scores: Some(scores),
        involvement,
    }))
}
