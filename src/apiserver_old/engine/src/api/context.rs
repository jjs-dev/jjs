use super::{
    security::{AccessChecker, Subjects, Token, TokenMgr, TokenMgrError},
    ApiResult, ResultToApiUtil as _,
};
use std::sync::Arc;

pub(crate) type DbPool = Arc<db::DbConn>;

//TODO: Do not clone Context on every request
pub(crate) struct ContextData {
    pub(crate) cfg: Arc<entity::Loader>,
    pub(crate) as_cfg: Arc<crate::config::ApiserverParams>,
    pub(crate) token_mgr: TokenMgr,
    pub(crate) token: Token,
    pub(crate) problem_loader: Arc<problem_loader::Loader>,
    pub(crate) data_dir: Arc<std::path::Path>,
}

async fn append_participation_to_subjects(
    ctx: &Arc<ContextData>,
    subj: &mut Subjects,
) -> ApiResult<()> {
    let participation = ctx
        .load_participation(&subj.contest.as_ref().unwrap().id)
        .await?;
    subj.participation = participation;
    Ok(())
}

impl ContextData {
    async fn build_subjects_for_contest(
        self: &Arc<Self>,
        contest_id: &str,
    ) -> ApiResult<Option<Subjects>> {
        match self.cfg.find::<entity::Contest>(contest_id) {
            Some(c) => {
                let mut subjs = Subjects {
                    contest: Some(c.clone()),
                    run: None,
                    participation: None,
                };
                append_participation_to_subjects(self, &mut subjs).await?;
                Ok(Some(subjs))
            }
            None => Ok(None),
        }
    }

    async fn build_subjects_for_run(
        self: &Arc<Self>,
        run_id: db::schema::RunId,
    ) -> ApiResult<Option<Subjects>> {
        let run = match self
            .db()
            .run_try_load(run_id)
            .await
            .internal(&Context(self.clone()))?
        {
            Some(r) => r,
            None => return Ok(None),
        };
        let contest = match self.cfg.find::<entity::Contest>(&run.contest_id) {
            Some(contest) => contest.clone(),
            None => return Ok(None),
        };
        Ok(Some(Subjects {
            contest: Some(contest),
            run: Some(run),
            // not needed so we don't look up for it
            participation: None,
        }))
    }

    pub(crate) async fn load_participation(
        self: &Arc<Self>,
        contest_id: &str,
    ) -> ApiResult<Option<db::schema::Participation>> {
        self.db()
            .part_lookup(self.token.user_id(), &contest_id)
            .await
            .internal(&Context(self.clone()))
    }

    pub(crate) async fn access_contest<'a>(
        self: &'a Arc<Self>,
        contest_id: &str,
    ) -> ApiResult<Option<AccessChecker<'a>>> {
        let subjects = match self.build_subjects_for_contest(contest_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };
        Ok(Some(AccessChecker {
            token: &self.token,
            cfg: &self.cfg,
            subjects,
        }))
    }

    pub(crate) async fn access_run<'a>(
        self: &'a Arc<Self>,
        run_id: db::schema::RunId,
    ) -> ApiResult<Option<AccessChecker<'a>>> {
        let subjects = match self.build_subjects_for_run(run_id).await? {
            Some(s) => s,
            None => return Ok(None),
        };
        Ok(Some(AccessChecker {
            token: &self.token,
            cfg: &self.cfg,
            subjects: (subjects),
        }))
    }

    pub(crate) fn access_generic(&self) -> AccessChecker {
        AccessChecker {
            token: &self.token,
            cfg: &self.cfg,
            subjects: (Subjects {
                contest: None,
                run: None,
                participation: None,
            }),
        }
    }

    pub(crate) fn db(&self) -> &db::DbConn {
        &*self.as_cfg.db_conn
    }

    pub(crate) fn config(&self) -> &crate::config::ApiserverConfig {
        &self.as_cfg.cfg
    }
}

#[derive(Clone)]
pub(crate) struct Context(pub(crate) Arc<ContextData>);

impl std::ops::Deref for Context {
    type Target = Arc<ContextData>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[rocket::async_trait]
impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for ContextData {
    type Error = TokenMgrError;

    async fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let factory: rocket::State<ContextFactory> = request
            .guard::<rocket::State<ContextFactory>>()
            .await
            .expect("State<ContextFactory> missing");

        let apiserver_config = request
            .guard::<rocket::State<Arc<crate::config::ApiserverParams>>>()
            .await
            .expect("State<Arc<ApiserverParams>> missing");

        let secret_key = request
            .guard::<rocket::State<crate::secret_key::SecretKey>>()
            .await
            .expect("State<SecretKey> missing");

        let token = request
            .headers()
            .get("Authorization")
            .next()
            .ok_or(TokenMgrError::TokenMissing);

        let secret_key = Arc::clone(&(*secret_key).0);
        let token_mgr = TokenMgr::new(factory.pool.clone(), secret_key);
        let token = match token {
            Ok(header) => {
                token_mgr
                    .deserialize(header.as_bytes(), apiserver_config.cfg.env.is_dev())
                    .await
            }
            Err(err) => Err(err),
        };

        let token = match token {
            Ok(tok) => tok,
            Err(e) => match e {
                TokenMgrError::TokenMissing => match token_mgr.create_guest_token().await {
                    Ok(guest_token) => guest_token,
                    Err(err) => {
                        return rocket::request::Outcome::Failure((
                            rocket::http::Status::InternalServerError,
                            err,
                        ));
                    }
                },
                _ => {
                    return rocket::request::Outcome::Failure((
                        rocket::http::Status::BadRequest,
                        e,
                    ));
                }
            },
        };

        rocket::Outcome::Success(ContextData {
            cfg: factory.cfg.clone(),
            as_cfg: apiserver_config.clone(),
            token_mgr,
            token,
            problem_loader: factory.problem_loader.clone(),
            data_dir: factory.data_dir.clone(),
        })
    }
}
#[rocket::async_trait]
impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for Context {
    type Error = TokenMgrError;

    async fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let context_data = match request.guard::<ContextData>().await {
            rocket::Outcome::Success(data) => data,
            rocket::Outcome::Failure(fail) => return rocket::Outcome::Failure(fail),
            rocket::Outcome::Forward(()) => return rocket::Outcome::Forward(()),
        };
        let ctx = Context(Arc::new(context_data));
        rocket::Outcome::Success(ctx)
    }
}

pub(crate) struct ContextFactory {
    pub(crate) pool: DbPool,
    pub(crate) cfg: Arc<entity::Loader>,
    pub(crate) problem_loader: Arc<problem_loader::Loader>,
    pub(crate) data_dir: Arc<std::path::Path>,
}
