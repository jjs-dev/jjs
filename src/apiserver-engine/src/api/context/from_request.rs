use super::{
    ConfigContext, CredentialsContext, DbContext, EntityContext, SecurityContext,
    TokenManageContext,
};
use actix_web::{dev, FromRequest, HttpRequest};
use futures::future::TryFutureExt;
use std::rc::Rc;

impl FromRequest for TokenManageContext {
    type Config = ();
    type Error = actix_web::Error;

    type Future = impl std::future::Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        futures::future::ok(Self {
            mgr: req.app_data::<crate::api::TokenMgr>().unwrap().clone(),
        })
    }
}

impl FromRequest for DbContext {
    type Config = ();
    type Error = actix_web::Error;

    type Future = impl std::future::Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        futures::future::ok(Self {
            db_connection: req.app_data::<db::DbConn>().unwrap().clone(),
        })
    }
}

impl FromRequest for EntityContext {
    type Config = ();
    type Error = actix_web::Error;

    type Future = impl std::future::Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        futures::future::ok(Self {
            entity_loader: req.app_data::<entity::Loader>().unwrap().clone(),
            problem_loader: req.app_data::<problem_loader::Loader>().unwrap().clone(),
        })
    }
}

async fn cred_cx_from_req_inner(
    tm_cx: TokenManageContext,
    header: Option<Vec<u8>>,
    cf_cx: ConfigContext,
) -> Result<CredentialsContext, actix_web::Error> {
    let token = match header {
        Some(header) => {
            match tm_cx
                .token_manager()
                .deserialize(&header, cf_cx.config().env.is_dev())
                .await
            {
                Ok(tok) => tok,
                Err(err) => return Err(actix_web::error::ErrorBadRequest(err)),
            }
        }
        None => match tm_cx.token_manager().create_guest_token().await {
            Ok(tok) => tok,
            Err(err) => return Err(actix_web::error::ErrorInternalServerError(err)),
        },
    };
    Ok(CredentialsContext {
        token: Rc::new(token),
    })
}

impl FromRequest for CredentialsContext {
    type Config = ();
    type Error = actix_web::Error;

    type Future = impl std::future::Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        let header = req
            .headers()
            .get("Authorization")
            .map(|header_value| header_value.as_bytes().to_vec());
        let tm_cx_fut = TokenManageContext::from_request(req, payload);
        let cf_cx_fut = ConfigContext::from_request(req, payload);

        futures::future::try_join(tm_cx_fut, cf_cx_fut).and_then(|(tm_cx, cf_cx)| {
            cred_cx_from_req_inner(tm_cx, header, cf_cx)
            //  Ok(Self {token: Rc::new(token)})
        })
    }
}

impl FromRequest for SecurityContext {
    type Config = ();
    type Error = actix_web::Error;

    type Future = impl std::future::Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut dev::Payload) -> Self::Future {
        let authorizer = req
            .app_data::<crate::api::security::Authorizer>()
            .unwrap()
            .clone();
        CredentialsContext::from_request(req, payload).map_ok(|cred_cx| Self {
            cred_cx,
            authorizer,
        })
    }
}

impl FromRequest for ConfigContext {
    type Config = ();
    type Error = actix_web::Error;

    type Future = impl std::future::Future<Output = Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _: &mut dev::Payload) -> Self::Future {
        futures::future::ok(Self {
            apiserver_config: req
                .app_data::<Rc<crate::config::ApiserverConfig>>()
                .unwrap()
                .clone(),
            data_dir: req.app_data::<Rc<std::path::Path>>().unwrap().clone(),
        })
    }
}
