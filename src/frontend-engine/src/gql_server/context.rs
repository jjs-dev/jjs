use crate::security::{AccessChecker, Token, TokenMgr, TokenMgrError};
use std::sync::{Arc, Mutex};

pub(crate) type DbPool = Arc<dyn db::DbConn>;

//TODO: Do not clone Context on every request
pub(crate) struct ContextData {
    pub(crate) db: DbPool,
    pub(crate) cfg: Arc<cfg::Config>,
    pub(crate) fr_cfg: Arc<crate::config::FrontendConfig>,
    pub(crate) token_mgr: TokenMgr,
    pub(crate) token: Token,
    global: Arc<Mutex<crate::global::GlobalState>>,
}

impl ContextData {
    pub(crate) fn access(&self) -> AccessChecker {
        AccessChecker {
            token: &self.token,
            cfg: &self.cfg,
            db: &*self.db,
        }
    }

    pub(crate) fn global(&self) -> std::sync::MutexGuard<crate::global::GlobalState> {
        self.global.lock().unwrap()
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

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for ContextData {
    type Error = TokenMgrError;

    fn from_request(
        request: &'a rocket::request::Request<'r>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let factory: rocket::State<ContextFactory> = request
            .guard::<rocket::State<ContextFactory>>()
            .expect("State<ContextFactory> missing");

        let frontend_config = request
            .guard::<rocket::State<Arc<crate::config::FrontendConfig>>>()
            .expect("State<Arc<FrontendConfig>> missing");

        let secret_key = request
            .guard::<rocket::State<crate::security::SecretKey>>()
            .expect("State<SecretKey> missing");

        let token = request
            .headers()
            .get("X-Jjs-Auth")
            .next()
            .ok_or(TokenMgrError::TokenMissing);

        let global = request
            .guard::<rocket::State<Arc<Mutex<crate::global::GlobalState>>>>()
            .expect("State<Arc<Mutex<Global>>> missing");

        let secret_key = Arc::clone(&(*secret_key).0);
        let token_mgr = TokenMgr::new(factory.pool.clone(), secret_key);

        let token = token.and_then(|header| {
            token_mgr.deserialize(header.as_bytes(), frontend_config.env.is_dev())
        });
        let token = match token {
            Ok(tok) => Ok(tok),
            Err(e) => match e {
                TokenMgrError::TokenMissing => Ok(token_mgr.create_guest_token())
                    .map_err(|e| Err((rocket::http::Status::BadRequest, e)))?,
                _ => Err(e),
            },
        };
        let token = token.map_err(|e| Err((rocket::http::Status::BadRequest, e)))?;

        rocket::Outcome::Success(ContextData {
            db: factory.pool.clone(),
            cfg: factory.cfg.clone(),
            fr_cfg: frontend_config.clone(),
            token_mgr,
            token,
            global: (*global).clone(),
        })
    }
}

impl<'a, 'r> rocket::request::FromRequest<'a, 'r> for Context {
    type Error = TokenMgrError;

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
    pub(crate) fr_cfg: Arc<crate::config::FrontendConfig>,
}

impl ContextFactory {
    /// Creates context, not bound to particular request
    pub(crate) fn create_context_data_unrestricted(&self) -> ContextData {
        let secret_key = Arc::new([]);
        let token_mgr = TokenMgr::new(self.pool.clone(), secret_key);
        let token = match token_mgr.create_root_token() {
            Ok(tok) => tok,
            Err(e) => panic!("failed create root's Token: {}", e),
        };
        ContextData {
            db: self.pool.clone(),
            cfg: self.cfg.clone(),
            token_mgr,
            token,
            global: Arc::new(Mutex::new(crate::global::GlobalState::new())),
            fr_cfg: self.fr_cfg.clone(),
        }
    }
}

impl juniper::Context for Context {}
