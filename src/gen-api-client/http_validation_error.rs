#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HttpValidationError {
    pub detail: Option<Vec<crate::validation_error::ValidationError>>,
}

impl HttpValidationError {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> HttpValidationErrorBuilder {
        HttpValidationErrorBuilder {
            body: Default::default(),
        }
    }
}

impl Into<HttpValidationError> for HttpValidationErrorBuilder {
    fn into(self) -> HttpValidationError {
        self.body
    }
}

/// Builder for [`HttpValidationError`](./struct.HttpValidationError.html) object.
#[derive(Debug, Clone)]
pub struct HttpValidationErrorBuilder {
    body: self::HttpValidationError,
}

impl HttpValidationErrorBuilder {
    #[inline]
    pub fn detail(mut self, value: impl Iterator<Item = crate::validation_error::ValidationErrorBuilder<crate::generics::LocExists, crate::generics::MsgExists, crate::generics::TypeExists>>) -> Self {
        self.body.detail = Some(value.map(|value| value.into()).collect::<Vec<_>>().into());
        self
    }
}
