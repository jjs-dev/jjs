pub(crate) mod auth;
pub(crate) mod contests;
mod context;
pub(crate) mod misc;
pub(crate) mod monitor;
pub(crate) mod runs;
mod schema;
mod security;
pub(crate) mod toolchains;
pub(crate) mod users;

#[derive(Debug)]
struct ErrorExtension(serde_json::Map<String, serde_json::Value>);

impl ErrorExtension {
    const KEY_DEV_BACKTRACE: &'static str = "trace";
    const KEY_DEV_ERROR_BACKTRACE: &'static str = "errorTrace";
    const KEY_ERROR_CODE: &'static str = "errorCode";

    fn new() -> Self {
        Self(serde_json::Map::new())
    }

    fn set_backtrace(&mut self) {
        let trace = backtrace::Backtrace::new();

        let trace = format!("{:?}", trace);

        self.0.insert(
            Self::KEY_DEV_BACKTRACE.to_string(),
            serde_json::Value::String(trace),
        );
    }

    fn set_error_code(&mut self, error_code: &str) {
        self.0.insert(
            Self::KEY_ERROR_CODE.to_string(),
            serde_json::Value::String(error_code.to_string()),
        );
    }

    fn into_value(self) -> serde_json::Value {
        serde_json::Value::Object(self.0)
    }
}

pub(crate) struct ApiError {
    visible: bool,
    extension: ErrorExtension,
    cause: Option<anyhow::Error>,
    ctx: Context,
}

impl ApiError {
    fn dev_backtrace(&mut self) {
        if self.ctx.config().env.is_dev() {
            self.extension.set_backtrace();
            if let Some(err) = &self.cause {
                let backtrace = format!("{:?}", err.backtrace());
                self.extension.0.insert(
                    ErrorExtension::KEY_DEV_ERROR_BACKTRACE.to_string(),
                    serde_json::Value::String(backtrace),
                );
            }
        }
    }

    pub fn new(ctx: &Context, error_code: &str) -> Self {
        let mut extension = ErrorExtension::new();
        extension.set_error_code(error_code);
        let mut s = Self {
            visible: true,
            extension,
            cause: None,
            ctx: ctx.clone(),
        };
        s.dev_backtrace();
        s
    }

    pub fn access_denied(ctx: &Context) -> Self {
        Self::new(ctx, "AccessDenied")
    }

    pub fn not_found(ctx: &Context) -> Self {
        Self::new(ctx, "NotFound")
    }

    pub fn not_implemented(ctx: &Context) -> Self {
        Self::new(ctx, "NotImplemented")
    }

    fn is_visible(&self) -> bool {
        self.visible || self.ctx.config().env.is_dev()
    }
}

mod impl_display {
    use super::*;
    use std::fmt::{self, Display, Formatter};

    impl Display for ApiError {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "Apiserver error")?;
            if self.visible {
                write!(f, "(pub) ")?;
            } else {
                write!(f, "(priv) ")?;
            }

            write!(f, "[{:?}]", &self.extension)?;

            if let Some(src) = &self.cause {
                write!(f, ": {:#}", src)?;
            }
            Ok(())
        }
    }
}

#[derive(Debug)]
struct EmptyError;

impl std::fmt::Display for EmptyError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("internal error")
    }
}

impl std::error::Error for EmptyError {}

struct AnyhowAlternateWrapper<'a>(&'a anyhow::Error);
impl std::fmt::Display for AnyhowAlternateWrapper<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{:#}", &self.0)
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[rocket::async_trait]
impl<'req> rocket::response::Responder<'req> for ApiError {
    async fn respond_to<'a>(
        self,
        _request: &'req rocket::request::Request<'a>,
    ) -> rocket::response::Result<'req> {
        let mut builder = rocket::response::Response::build();
        builder.status(rocket::http::Status::BadRequest);
        let error = match self.cause {
            Some(err) if self.is_visible() => (format!("{:#}", err)),
            _ => "error".to_string(),
        };
        let value = serde_json::json!({
            "detail": self.extension.into_value(),
            "message": error,
            "error": true,
        });
        let value = serde_json::to_vec(&value).expect("failed to serialize error");
        builder.header(rocket::http::ContentType::JSON);
        builder.sized_body(std::io::Cursor::new(value)).await;
        Ok(builder.finalize())
    }
}

trait ResultToApiUtil<T, E> {
    /// Handle error as internal, if any
    fn internal(self, ctx: &Context) -> Result<T, ApiError>;

    /// Show error to user, if any
    fn report(self, ctx: &Context) -> Result<T, ApiError>;

    /// like `report`, but also return extension
    fn report_ext(self, ctx: &Context, ext: ErrorExtension) -> Result<T, ApiError>;

    /// like 'report_ext', but produce extension from error with supplied callback
    fn report_with(
        self,
        ctx: &Context,
        make_ext: impl FnOnce(&E) -> ErrorExtension,
    ) -> Result<T, ApiError>;
}

impl<T, E: Into<anyhow::Error>> ResultToApiUtil<T, E> for Result<T, E> {
    fn internal(self, ctx: &Context) -> Result<T, ApiError> {
        self.map_err(|err| ApiError {
            visible: false,
            extension: ErrorExtension::new(),
            cause: Some(err.into()),
            ctx: ctx.clone(),
        })
        .map_err(|mut err| {
            err.dev_backtrace();
            err
        })
    }

    fn report(self, ctx: &Context) -> Result<T, ApiError> {
        self.report_ext(ctx, ErrorExtension::new())
    }

    fn report_ext(self, ctx: &Context, ext: ErrorExtension) -> Result<T, ApiError> {
        self.report_with(ctx, |_| ext)
    }

    fn report_with(
        self,
        ctx: &Context,
        make_ext: impl FnOnce(&E) -> ErrorExtension,
    ) -> Result<T, ApiError> {
        self.map_err(|err| ApiError {
            visible: true,
            extension: make_ext(&err),
            cause: Some(err.into()),
            ctx: ctx.clone(),
        })
        .map_err(|mut err| {
            err.dev_backtrace();
            err
        })
    }
}

pub(crate) trait ApiObject:
    serde::ser::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema
{
    fn name() -> &'static str;
}

mod prelude {
    pub(super) use super::{
        schema, ApiError, ApiObject, ApiResult, Context, ErrorExtension, ResultToApiUtil as _,
    };
    pub(super) use rocket::{delete, get, patch, post};
    pub(super) use rocket_contrib::json::Json;
    pub(super) use schemars::JsonSchema;
    pub(super) use serde::{Deserialize, Serialize};
}

pub(crate) use context::{Context, ContextFactory};
pub use security::{TokenMgr, TokenMgrError};
