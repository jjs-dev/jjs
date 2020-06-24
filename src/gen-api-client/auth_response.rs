#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

impl AuthResponse {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> AuthResponseBuilder<crate::generics::MissingToken> {
        AuthResponseBuilder {
            body: Default::default(),
            _token: core::marker::PhantomData,
        }
    }
}

impl Into<AuthResponse> for AuthResponseBuilder<crate::generics::TokenExists> {
    fn into(self) -> AuthResponse {
        self.body
    }
}

/// Builder for [`AuthResponse`](./struct.AuthResponse.html) object.
#[derive(Debug, Clone)]
pub struct AuthResponseBuilder<Token> {
    body: self::AuthResponse,
    _token: core::marker::PhantomData<Token>,
}

impl<Token> AuthResponseBuilder<Token> {
    #[inline]
    pub fn token(mut self, value: impl Into<String>) -> AuthResponseBuilder<crate::generics::TokenExists> {
        self.body.token = value.into();
        unsafe { std::mem::transmute(self) }
    }
}
