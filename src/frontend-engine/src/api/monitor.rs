use super::prelude::*;
use std::{collections::BTreeMap, convert::TryInto, num::NonZeroU32};

fn lower_run(_run: &db::schema::Run, inv: &invoker_api::InvokeOutcomeHeader) -> ranker::Run {
    let mut subtasks = BTreeMap::new();
    //let outcome_header = inv.o
    subtasks.insert(
        ranker::SubtaskId(NonZeroU32::new(1).unwrap()),
        inv.score.unwrap_or(0) as i32,
    );
    // TODO: properly support subtasks
    // TODO: keep party info for runs
    // TODO: keep problem_id for runs
    ranker::Run {
        subtasks,
        party: ranker::PartyId(NonZeroU32::new(1).unwrap()),
        problem: ranker::ProblemId(NonZeroU32::new(1).unwrap()),
    }
}

fn lower_problem(prob: &pom::Problem) -> ranker::ProblemConfig {
    // TODO: get all this stuff from problem config
    ranker::ProblemConfig {
        name: prob.title.to_string(),
        accepted_score: 100,
        score_runs: ranker::RunScoreAggregationTarget::Best,
        aggregation: ranker::RunScoreAggregation::Max,
    }
}

pub(super) fn get_standings(ctx: &Context) -> ApiResult<String> {
    let runs = ctx.db.load_runs_with_last_invocations().internal(ctx)?;

    let mut ranker_runs = Vec::new();
    for (db_run, db_inv) in runs {
        let headers = db_inv.invoke_outcome_headers().internal(ctx)?;
        let header = headers
            .into_iter()
            .find(|header| header.kind == invoker_api::valuer_proto::JudgeLogKind::Contestant);
        let header = match header {
            Some(h) => h,
            None => continue,
        };
        ranker_runs.push(lower_run(&db_run, &header));
    }
    let mut ranker_problems = ctx
        .problem_loader
        .list()
        .map(|problem| (lower_problem(problem.0)))
        .collect::<Vec<_>>();

    ranker_problems.sort_by(|lhs, rhs| lhs.name.cmp(&rhs.name));

    let mut ranker_problems_with_id = Vec::new();
    for (i, prob_cfg) in ranker_problems.into_iter().map(|x| x).enumerate() {
        let id = NonZeroU32::new((i + 1).try_into().unwrap()).unwrap();
        ranker_problems_with_id.push((ranker::ProblemId(id), prob_cfg));
    }

    let ranker_config = ranker::Config {
        penalty_aggregation: ranker::PenaltyAggregation::Sum,
        score_problems: ranker::ProblemScoreAggregationTarget::All,
    };

    let monitor = ranker::build_monitor(
        &ranker_runs,
        &ranker_problems_with_id,
        &[ranker::PartyId(NonZeroU32::new(1).unwrap())],
        &ranker_config,
    );

    Ok(serde_json::to_string(&monitor).unwrap())
}
