#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UserCreationParams {
    pub login: String,
    pub password: String,
    pub roles: Vec<String>,
}

impl UserCreationParams {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> UserCreationParamsBuilder<crate::generics::MissingLogin, crate::generics::MissingPassword, crate::generics::MissingRoles> {
        UserCreationParamsBuilder {
            body: Default::default(),
            _login: core::marker::PhantomData,
            _password: core::marker::PhantomData,
            _roles: core::marker::PhantomData,
        }
    }

    /// Creates new user
    #[inline]
    pub fn create_user() -> UserCreationParamsPostBuilder<crate::generics::MissingLogin, crate::generics::MissingPassword, crate::generics::MissingRoles> {
        UserCreationParamsPostBuilder {
            body: Default::default(),
            _login: core::marker::PhantomData,
            _password: core::marker::PhantomData,
            _roles: core::marker::PhantomData,
        }
    }
}

impl Into<UserCreationParams> for UserCreationParamsBuilder<crate::generics::LoginExists, crate::generics::PasswordExists, crate::generics::RolesExists> {
    fn into(self) -> UserCreationParams {
        self.body
    }
}

impl Into<UserCreationParams> for UserCreationParamsPostBuilder<crate::generics::LoginExists, crate::generics::PasswordExists, crate::generics::RolesExists> {
    fn into(self) -> UserCreationParams {
        self.body
    }
}

/// Builder for [`UserCreationParams`](./struct.UserCreationParams.html) object.
#[derive(Debug, Clone)]
pub struct UserCreationParamsBuilder<Login, Password, Roles> {
    body: self::UserCreationParams,
    _login: core::marker::PhantomData<Login>,
    _password: core::marker::PhantomData<Password>,
    _roles: core::marker::PhantomData<Roles>,
}

impl<Login, Password, Roles> UserCreationParamsBuilder<Login, Password, Roles> {
    #[inline]
    pub fn login(mut self, value: impl Into<String>) -> UserCreationParamsBuilder<crate::generics::LoginExists, Password, Roles> {
        self.body.login = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn password(mut self, value: impl Into<String>) -> UserCreationParamsBuilder<Login, crate::generics::PasswordExists, Roles> {
        self.body.password = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn roles(mut self, value: impl Iterator<Item = impl Into<String>>) -> UserCreationParamsBuilder<Login, Password, crate::generics::RolesExists> {
        self.body.roles = value.map(|value| value.into()).collect::<Vec<_>>().into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`UserCreationParams::create_user`](./struct.UserCreationParams.html#method.create_user) method for a `POST` operation associated with `UserCreationParams`.
#[derive(Debug, Clone)]
pub struct UserCreationParamsPostBuilder<Login, Password, Roles> {
    body: self::UserCreationParams,
    _login: core::marker::PhantomData<Login>,
    _password: core::marker::PhantomData<Password>,
    _roles: core::marker::PhantomData<Roles>,
}

impl<Login, Password, Roles> UserCreationParamsPostBuilder<Login, Password, Roles> {
    #[inline]
    pub fn login(mut self, value: impl Into<String>) -> UserCreationParamsPostBuilder<crate::generics::LoginExists, Password, Roles> {
        self.body.login = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn password(mut self, value: impl Into<String>) -> UserCreationParamsPostBuilder<Login, crate::generics::PasswordExists, Roles> {
        self.body.password = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn roles(mut self, value: impl Iterator<Item = impl Into<String>>) -> UserCreationParamsPostBuilder<Login, Password, crate::generics::RolesExists> {
        self.body.roles = value.map(|value| value.into()).collect::<Vec<_>>().into();
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for UserCreationParamsPostBuilder<crate::generics::LoginExists, crate::generics::PasswordExists, crate::generics::RolesExists> {
    type Output = crate::user::User;

    const METHOD: http::Method = http::Method::POST;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/users".into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .json(&self.body))
    }
}
