pub(crate) mod auth;
pub(crate) mod contests;
pub(crate) mod context;
pub(crate) mod misc;
pub(crate) mod monitor;
mod prelude;
pub(crate) mod runs;
mod schema;
pub mod security;
pub(crate) mod toolchains;
pub(crate) mod users;

use log::warn;

pub(crate) struct ApiError {
    visible: bool,
    extension: ErrorExtension,
    cause: Option<anyhow::Error>,
}

pub use security::TokenMgr;

#[derive(Debug, Clone)]
struct ErrorExtension(serde_json::Map<String, serde_json::Value>);

impl ErrorExtension {
    const KEY_ERROR_CODE: &'static str = "errorCode";

    fn new() -> Self {
        Self(serde_json::Map::new())
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

impl std::fmt::Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiError")
            .field("visible", &self.visible)
            .field("extension", &self.extension)
            .field("cause", &self.cause)
            .finish()
    }
}

impl ApiError {
    pub fn new(error_code: &str) -> Self {
        let mut extension = ErrorExtension::new();
        extension.set_error_code(error_code);
        ApiError {
            visible: true,
            extension,
            cause: None,
        }
    }

    pub fn access_denied() -> Self {
        Self::new("AccessDenied")
    }

    pub fn not_found() -> Self {
        Self::new("NotFound")
    }

    pub fn not_implemented() -> Self {
        Self::new("NotImplemented")
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

impl actix_web::error::ResponseError for ApiError {
    fn status_code(&self) -> actix_web::http::StatusCode {
        match self
            .extension
            .0
            .get("errorCode")
            .and_then(|val| val.as_str())
        {
            Some("NotFound") => actix_web::http::StatusCode::NOT_FOUND,
            Some("AccessDenied") => actix_web::http::StatusCode::FORBIDDEN,
            _ => actix_web::http::StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> actix_web::web::HttpResponse {
        let mut resp = actix_web::web::HttpResponse::new(self.status_code());
        let error = match &self.cause {
            Some(err) if self.visible => (format!("{:#}", err)),
            Some(err) => {
                warn!("internal error: {:#}", err);
                "unexpected error".to_string()
            }
            None => "unexpected error".to_string(),
        };
        let value = serde_json::json!({
            "detail": self.extension.clone().into_value(),
            "message": error,
            "error": true,
        });
        let value = serde_json::to_vec(&value).expect("failed to serialize error");
        resp.headers_mut().insert(
            actix_web::http::header::CONTENT_TYPE,
            actix_web::http::header::HeaderValue::from_static("application/json"),
        );

        resp.set_body(actix_web::body::Body::from(value))
    }
}

trait ResultToApiUtil<T, E> {
    /// Handle error as internal, if any
    fn internal(self) -> Result<T, ApiError>;

    /// Show error to user, if any
    fn report(self) -> Result<T, ApiError>;

    /// like `report`, but also return extension
    fn report_ext(self, ext: ErrorExtension) -> Result<T, ApiError>;

    /// like 'report_ext', but produce extension from error with supplied
    /// callback
    fn report_with(self, make_ext: impl FnOnce(&E) -> ErrorExtension) -> Result<T, ApiError>;
}

impl<T, E: Into<anyhow::Error>> ResultToApiUtil<T, E> for Result<T, E> {
    fn internal(self) -> Result<T, ApiError> {
        self.map_err(|err| ApiError {
            visible: false,
            extension: ErrorExtension::new(),
            cause: Some(err.into()),
        })
    }

    fn report(self) -> Result<T, ApiError> {
        self.report_ext(ErrorExtension::new())
    }

    fn report_ext(self, ext: ErrorExtension) -> Result<T, ApiError> {
        self.report_with(|_| ext)
    }

    fn report_with(self, make_ext: impl FnOnce(&E) -> ErrorExtension) -> Result<T, ApiError> {
        self.map_err(|err| ApiError {
            visible: true,
            extension: make_ext(&err),
            cause: Some(err.into()),
        })
    }
}

pub(crate) trait ApiObject:
    serde::ser::Serialize + serde::de::DeserializeOwned + schemars::JsonSchema
{
    fn name() -> &'static str;
}

struct EmptyResponse;

// TODO: use ! instead
enum NeverError {}

impl std::fmt::Display for NeverError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl std::fmt::Debug for NeverError {
    fn fmt(&self, _: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl actix_web::ResponseError for NeverError {}

impl actix_web::Responder for EmptyResponse {
    type Error = NeverError;
    type Future = futures::future::Ready<Result<actix_web::HttpResponse, NeverError>>;

    fn respond_to(self, _: &actix_web::HttpRequest) -> Self::Future {
        futures::future::ok(actix_web::HttpResponse::NoContent().finish())
    }
}

#[macro_export]
macro_rules! make_conditions {
    () => {
        anymap::AnyMap::new()
    };
    ($val: expr) => {{
        let mut m = anymap::AnyMap::new();
        m.insert($val);
        m
    }};
    ($val: expr, $($tail:expr),*) => {{
        let mut m = make_conditions!($($tail),*);
        m.insert($val);
    }};
}
