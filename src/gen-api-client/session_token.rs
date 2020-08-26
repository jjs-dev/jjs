#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub data: String,
}

impl SessionToken {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> SessionTokenBuilder<crate::generics::MissingData> {
        SessionTokenBuilder {
            body: Default::default(),
            _data: core::marker::PhantomData,
        }
    }
}

impl Into<SessionToken> for SessionTokenBuilder<crate::generics::DataExists> {
    fn into(self) -> SessionToken {
        self.body
    }
}

/// Builder for [`SessionToken`](./struct.SessionToken.html) object.
#[derive(Debug, Clone)]
pub struct SessionTokenBuilder<Data> {
    body: self::SessionToken,
    _data: core::marker::PhantomData<Data>,
}

impl<Data> SessionTokenBuilder<Data> {
    #[inline]
    pub fn data(mut self, value: impl Into<String>) -> SessionTokenBuilder<crate::generics::DataExists> {
        self.body.data = value.into();
        unsafe { std::mem::transmute(self) }
    }
}
