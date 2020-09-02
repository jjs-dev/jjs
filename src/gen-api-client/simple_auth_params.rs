#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SimpleAuthParams {
    pub login: String,
    pub password: String,
}

impl SimpleAuthParams {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> SimpleAuthParamsBuilder<crate::generics::MissingLogin, crate::generics::MissingPassword> {
        SimpleAuthParamsBuilder {
            body: Default::default(),
            _login: core::marker::PhantomData,
            _password: core::marker::PhantomData,
        }
    }

    /// Login using login and password
    ///
    /// In future, other means to authn will be added.
    #[inline]
    pub fn login() -> SimpleAuthParamsPostBuilder<crate::generics::MissingLogin, crate::generics::MissingPassword> {
        SimpleAuthParamsPostBuilder {
            body: Default::default(),
            _login: core::marker::PhantomData,
            _password: core::marker::PhantomData,
        }
    }
}

impl Into<SimpleAuthParams> for SimpleAuthParamsBuilder<crate::generics::LoginExists, crate::generics::PasswordExists> {
    fn into(self) -> SimpleAuthParams {
        self.body
    }
}

impl Into<SimpleAuthParams> for SimpleAuthParamsPostBuilder<crate::generics::LoginExists, crate::generics::PasswordExists> {
    fn into(self) -> SimpleAuthParams {
        self.body
    }
}

/// Builder for [`SimpleAuthParams`](./struct.SimpleAuthParams.html) object.
#[derive(Debug, Clone)]
pub struct SimpleAuthParamsBuilder<Login, Password> {
    body: self::SimpleAuthParams,
    _login: core::marker::PhantomData<Login>,
    _password: core::marker::PhantomData<Password>,
}

impl<Login, Password> SimpleAuthParamsBuilder<Login, Password> {
    #[inline]
    pub fn login(mut self, value: impl Into<String>) -> SimpleAuthParamsBuilder<crate::generics::LoginExists, Password> {
        self.body.login = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn password(mut self, value: impl Into<String>) -> SimpleAuthParamsBuilder<Login, crate::generics::PasswordExists> {
        self.body.password = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`SimpleAuthParams::login`](./struct.SimpleAuthParams.html#method.login) method for a `POST` operation associated with `SimpleAuthParams`.
#[derive(Debug, Clone)]
pub struct SimpleAuthParamsPostBuilder<Login, Password> {
    body: self::SimpleAuthParams,
    _login: core::marker::PhantomData<Login>,
    _password: core::marker::PhantomData<Password>,
}

impl<Login, Password> SimpleAuthParamsPostBuilder<Login, Password> {
    #[inline]
    pub fn login(mut self, value: impl Into<String>) -> SimpleAuthParamsPostBuilder<crate::generics::LoginExists, Password> {
        self.body.login = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn password(mut self, value: impl Into<String>) -> SimpleAuthParamsPostBuilder<Login, crate::generics::PasswordExists> {
        self.body.password = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for SimpleAuthParamsPostBuilder<crate::generics::LoginExists, crate::generics::PasswordExists> {
    type Output = crate::session_token::SessionToken;

    const METHOD: http::Method = http::Method::POST;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/auth/simple".into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .json(&self.body))
    }
}
