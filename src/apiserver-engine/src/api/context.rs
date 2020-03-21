use super::security::{RawAccessChecker, Token, TokenMgr, TokenMgrError};
use std::sync::{Arc, Mutex};

pub(crate) type DbPool = Arc<dyn db::DbConn>;

//TODO: Do not clone Context on every request
pub(crate) struct ContextData {
    pub(crate) cfg: Arc<entity::Loader>,
    pub(crate) as_cfg: Arc<crate::config::ApiserverParams>,
    pub(crate) token_mgr: TokenMgr,
    pub(crate) token: Token,
    pub(crate) problem_loader: Arc<problem_loader::Loader>,
    pub(crate) data_dir: Arc<std::path::Path>,
    global: Arc<Mutex<crate::global::GlobalState>>,
}

impl ContextData {
    pub(crate) fn access(&self) -> RawAccessChecker {
        RawAccessChecker {
            token: &self.token,
            cfg: &self.cfg,
            db: &*self.db(),
        }
    }

    pub(crate) fn global(&self) -> std::sync::MutexGuard<crate::global::GlobalState> {
        self.global.lock().unwrap()
    }

    pub(crate) fn db(&self) -> &dyn db::DbConn {
        &*self.as_cfg.db_conn
    }

    pub(crate) fn config(&self) -> &crate::config::ApiserverConfig {
        &self.as_cfg.cfg
    }
}

#[derive(Clone)]
pub(crate) struct Context(pub(crate) Arc<ContextData>);

impl std::ops::Deref for Context {
    type Target = ContextData;

    fn deref(&self) -> &Self::Target {
        &*self.0
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

        let global = request
            .guard::<rocket::State<Arc<Mutex<crate::global::GlobalState>>>>()
            .await
            .expect("State<Arc<Mutex<Global>>> missing");

        let secret_key = Arc::clone(&(*secret_key).0);
        let token_mgr = TokenMgr::new(factory.pool.clone(), secret_key);

        let token = token.and_then(|header| {
            token_mgr.deserialize(header.as_bytes(), apiserver_config.cfg.env.is_dev())
        });
        let token = match token {
            Ok(tok) => tok,
            Err(e) => match e {
                TokenMgrError::TokenMissing => match token_mgr.create_guest_token() {
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
            global: (*global).clone(),
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
