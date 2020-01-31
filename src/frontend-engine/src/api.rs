mod auth;
mod context;
mod misc;
mod monitor;
mod queries;
mod runs;
mod schema;
mod security;
mod users;
use slog_scope::error;

#[derive(Debug)]
struct ErrorExtension(juniper::Object<juniper::DefaultScalarValue>);

impl ErrorExtension {
    const KEY_DEV_BACKTRACE: &'static str = "trace";
    const KEY_DEV_ERROR_BACKTRACE: &'static str = "errorTrace";
    const KEY_ERROR_CODE: &'static str = "errorCode";

    fn new() -> Self {
        Self(juniper::Object::with_capacity(0))
    }

    fn set_backtrace(&mut self) {
        let trace = backtrace::Backtrace::new();

        let trace = format!("{:?}", trace);

        self.0.add_field(
            Self::KEY_DEV_BACKTRACE,
            juniper::Value::scalar(trace.as_str()),
        );
    }

    fn set_error_code(&mut self, error_code: &str) {
        self.0
            .add_field(Self::KEY_ERROR_CODE, juniper::Value::scalar(error_code));
    }

    fn into_value(self) -> juniper::Value<juniper::DefaultScalarValue> {
        juniper::Value::Object(self.0)
    }
}

struct ApiError {
    visible: bool,
    extension: ErrorExtension,
    source: Option<anyhow::Error>,
    ctx: Context,
}

impl ApiError {
    fn dev_backtrace(&mut self) {
        if self.ctx.fr_cfg.env.is_dev() {
            self.extension.set_backtrace();
            if let Some(err) = &self.source {
                let backtrace = format!("{:?}", err.backtrace());
                self.extension.0.add_field(
                    ErrorExtension::KEY_DEV_ERROR_BACKTRACE,
                    juniper::Value::scalar(backtrace),
                );
            }
        }
    }

    pub fn new(ctx: &Context, error_code: &str) -> Self {
        let mut ext = ErrorExtension::new();
        ext.set_error_code(error_code);
        let mut s = Self {
            visible: true,
            extension: ext,
            source: None,
            ctx: ctx.clone(),
        };
        s.dev_backtrace();
        s
    }

    pub fn access_denied(ctx: &Context) -> Self {
        Self::new(ctx, "AccessDenied")
    }
}

mod impl_display {
    use super::*;
    use std::fmt::{self, Display, Formatter};

    impl Display for ApiError {
        fn fmt(&self, f: &mut Formatter) -> fmt::Result {
            write!(f, "Frontend error")?;
            if self.visible {
                write!(f, "(pub) ")?;
            } else {
                write!(f, "(priv) ")?;
            }

            write!(f, "[{:?}]", &self.extension)?;

            if let Some(src) = &self.source {
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

impl juniper::IntoFieldError for ApiError {
    fn into_field_error(self) -> juniper::FieldError {
        let is_visible = self.visible || self.ctx.fr_cfg.env.is_dev();
        let wrp;
        let data: &dyn std::fmt::Display = match &self.source {
            Some(err) if is_visible => {
                wrp = AnyhowAlternateWrapper(err);
                &wrp
            }
            _ => {
                if let Some(err) = self.source {
                    let err_msg = err.to_string();
                    error!(
                        "Error when processing api request: {error}",
                        error = &err_msg
                    );
                }
                &EmptyError
            }
        };
        juniper::FieldError::new(data, self.extension.into_value())
    }
}

type ApiResult<T> = Result<T, ApiError>;

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
            source: Some(err.into()),
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
            source: Some(err.into()),
            ctx: ctx.clone(),
        })
        .map_err(|mut err| {
            err.dev_backtrace();
            err
        })
    }
}

trait StrErrorMsgUtil {
    fn report<T>(&self, ctx: &Context) -> Result<T, ApiError>;
}

impl StrErrorMsgUtil for str {
    fn report<T>(&self, ctx: &Context) -> Result<T, ApiError> {
        Err(ApiError {
            visible: true,
            extension: ErrorExtension::new(),
            source: Some(anyhow::anyhow!("{}", self)),
            ctx: ctx.clone(),
        })
    }
}

mod prelude {
    pub(super) use super::{
        schema, ApiError, ApiResult, Context, ErrorExtension, ResultToApiUtil as _,
        StrErrorMsgUtil as _,
    };
    pub(super) use juniper::{GraphQLInputObject, GraphQLObject};
}

pub(crate) use context::{Context, ContextFactory};
pub use security::{TokenMgr, TokenMgrError};

pub(crate) struct Query;

pub(crate) struct Mutation;

pub(crate) type Schema = juniper::RootNode<'static, Query, Mutation>;
