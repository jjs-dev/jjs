#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub login: String,
    pub password_hash: String,
    pub roles: Vec<String>,
}

impl User {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> UserBuilder<crate::generics::MissingId, crate::generics::MissingLogin, crate::generics::MissingPasswordHash, crate::generics::MissingRoles> {
        UserBuilder {
            body: Default::default(),
            _id: core::marker::PhantomData,
            _login: core::marker::PhantomData,
            _password_hash: core::marker::PhantomData,
            _roles: core::marker::PhantomData,
        }
    }
}

impl Into<User> for UserBuilder<crate::generics::IdExists, crate::generics::LoginExists, crate::generics::PasswordHashExists, crate::generics::RolesExists> {
    fn into(self) -> User {
        self.body
    }
}

/// Builder for [`User`](./struct.User.html) object.
#[derive(Debug, Clone)]
pub struct UserBuilder<Id, Login, PasswordHash, Roles> {
    body: self::User,
    _id: core::marker::PhantomData<Id>,
    _login: core::marker::PhantomData<Login>,
    _password_hash: core::marker::PhantomData<PasswordHash>,
    _roles: core::marker::PhantomData<Roles>,
}

impl<Id, Login, PasswordHash, Roles> UserBuilder<Id, Login, PasswordHash, Roles> {
    #[inline]
    pub fn id(mut self, value: impl Into<String>) -> UserBuilder<crate::generics::IdExists, Login, PasswordHash, Roles> {
        self.body.id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn login(mut self, value: impl Into<String>) -> UserBuilder<Id, crate::generics::LoginExists, PasswordHash, Roles> {
        self.body.login = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn password_hash(mut self, value: impl Into<String>) -> UserBuilder<Id, Login, crate::generics::PasswordHashExists, Roles> {
        self.body.password_hash = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn roles(mut self, value: impl Iterator<Item = impl Into<String>>) -> UserBuilder<Id, Login, PasswordHash, crate::generics::RolesExists> {
        self.body.roles = value.map(|value| value.into()).collect::<Vec<_>>().into();
        unsafe { std::mem::transmute(self) }
    }
}
