use super::{super::prelude::*, ContestId, ProblemId};

#[derive(GraphQLObject)]
pub(crate) struct Problem {
    /// Problem title as contestants see, e.g. "Find max flow".
    pub title: String,
    /// Problem external id (aka problem code) as contestants see. This is usually one letter or
    /// something similar, e.g. 'A' or '3F'.
    pub id: ProblemId,
}

pub(crate) struct Contest {
    pub title: String,
    pub id: ContestId,
}

#[juniper::object(Context = Context)]
impl Contest {
    /// E.g. "Berlandian Olympiad in Informatics. Finals. Day 3."
    fn title(&self) -> &str {
        &self.title
    }

    /// Configured by human, something readable like 'olymp-2019', or 'test-contest'
    fn id(&self) -> &str {
        &self.id
    }

    fn problems(&self, ctx: &Context) -> Vec<Problem> {
        let contest_cfg: &entity::Contest = ctx.cfg.find(&self.id).unwrap();
        contest_cfg
            .problems
            .iter()
            .map(|p| Problem {
                title: ctx
                    .problem_loader
                    .find(&p.name)
                    .expect("problem not found")
                    .0
                    .title
                    .clone(),
                id: p.code.clone(),
            })
            .collect()
    }
}
