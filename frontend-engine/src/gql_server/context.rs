use crate::security::TokenFromRequestError;
use std::sync::Arc;

pub(crate) type DbPool = Arc<dyn db::DbConn>;

//FIXME: Do not clone Context on every request
pub(crate) struct ContextData {
    pub(crate) db: DbPool,
    pub(crate) cfg: Arc<cfg::Config>,
    pub(crate) secret_key: Arc<[u8]>,
    pub(crate) env: crate::config::Env,
    pub(crate) logger: slog::Logger,
}

#[derive(Clone)]
pub(crate) struct Context(pub(crate) Arc<ContextData>);

impl std::ops::Deref for Context {
    type Target = ContextData;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for ContextData {
    type Error = TokenFromRequestError;

    fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let factory: rocket::State<ContextFactory> = request
            .guard::<rocket::State<ContextFactory>>()
            .expect("State<ContextFactory> missing");

        let env = request
            .guard::<rocket::State<crate::config::Env>>()
            .expect("State<Env> missing");

        let secret_key = request
            .guard::<rocket::State<crate::security::SecretKey>>()
            .expect("State<SecretKey> missing");

        rocket::Outcome::Success(ContextData {
            db: factory.pool.clone(),
            cfg: factory.cfg.clone(),
            env: *env,
            secret_key: Arc::clone(&(*secret_key).0),
            logger: factory.logger.clone(),
        })
    }
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for Context {
    type Error = TokenFromRequestError;

    fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let context_data = request.guard::<ContextData>()?;
        let ctx = Context(Arc::new(context_data));
        rocket::Outcome::Success(ctx)
    }
}

pub(crate) struct ContextFactory {
    pub(crate) pool: DbPool,
    pub(crate) cfg: Arc<cfg::Config>,
    pub(crate) logger: slog::Logger,
}

impl ContextFactory {
    /// Creates context, not bound to particular request
    pub(crate) fn create_context_data_unrestricted(&self) -> ContextData {
        ContextData {
            db: self.pool.clone(),
            cfg: self.cfg.clone(),
            secret_key: Arc::new([]),
            env: crate::config::Env::Dev,
            logger: self.logger.clone(),
        }
    }
}

impl juniper::Context for Context {}
