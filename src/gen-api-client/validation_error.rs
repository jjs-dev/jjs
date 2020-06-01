#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    pub loc: Vec<String>,
    pub msg: String,
    #[serde(rename = "type")]
    pub type_: String,
}

impl ValidationError {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> ValidationErrorBuilder<crate::generics::MissingLoc, crate::generics::MissingMsg, crate::generics::MissingType> {
        ValidationErrorBuilder {
            body: Default::default(),
            _loc: core::marker::PhantomData,
            _msg: core::marker::PhantomData,
            _type: core::marker::PhantomData,
        }
    }
}

impl Into<ValidationError> for ValidationErrorBuilder<crate::generics::LocExists, crate::generics::MsgExists, crate::generics::TypeExists> {
    fn into(self) -> ValidationError {
        self.body
    }
}

/// Builder for [`ValidationError`](./struct.ValidationError.html) object.
#[derive(Debug, Clone)]
pub struct ValidationErrorBuilder<Loc, Msg, Type> {
    body: self::ValidationError,
    _loc: core::marker::PhantomData<Loc>,
    _msg: core::marker::PhantomData<Msg>,
    _type: core::marker::PhantomData<Type>,
}

impl<Loc, Msg, Type> ValidationErrorBuilder<Loc, Msg, Type> {
    #[inline]
    pub fn loc(mut self, value: impl Iterator<Item = impl Into<String>>) -> ValidationErrorBuilder<crate::generics::LocExists, Msg, Type> {
        self.body.loc = value.map(|value| value.into()).collect::<Vec<_>>().into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn msg(mut self, value: impl Into<String>) -> ValidationErrorBuilder<Loc, crate::generics::MsgExists, Type> {
        self.body.msg = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn type_(mut self, value: impl Into<String>) -> ValidationErrorBuilder<Loc, Msg, crate::generics::TypeExists> {
        self.body.type_ = value.into();
        unsafe { std::mem::transmute(self) }
    }
}
