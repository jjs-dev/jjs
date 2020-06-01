#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ApiVersion {
    pub major: i64,
    pub minor: i64,
}

impl ApiVersion {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> ApiVersionBuilder<crate::generics::MissingMajor, crate::generics::MissingMinor> {
        ApiVersionBuilder {
            body: Default::default(),
            _major: core::marker::PhantomData,
            _minor: core::marker::PhantomData,
        }
    }

    /// Returns API version
    ///
    /// Version is returned in format {major: MAJOR, minor: MINOR}.
    /// MAJOR component is incremented, when backwards-incompatible changes were made.
    /// MINOR component is incremented, when backwards-compatible changes were made.
    ///
    /// It means, that if you tested application with apiVersion == X.Y, your application
    /// should assert that MAJOR = X and MINOR >= Y
    #[inline]
    pub fn api_version() -> ApiVersionGetBuilder {
        ApiVersionGetBuilder
    }
}

impl Into<ApiVersion> for ApiVersionBuilder<crate::generics::MajorExists, crate::generics::MinorExists> {
    fn into(self) -> ApiVersion {
        self.body
    }
}

/// Builder for [`ApiVersion`](./struct.ApiVersion.html) object.
#[derive(Debug, Clone)]
pub struct ApiVersionBuilder<Major, Minor> {
    body: self::ApiVersion,
    _major: core::marker::PhantomData<Major>,
    _minor: core::marker::PhantomData<Minor>,
}

impl<Major, Minor> ApiVersionBuilder<Major, Minor> {
    #[inline]
    pub fn major(mut self, value: impl Into<i64>) -> ApiVersionBuilder<crate::generics::MajorExists, Minor> {
        self.body.major = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn minor(mut self, value: impl Into<i64>) -> ApiVersionBuilder<Major, crate::generics::MinorExists> {
        self.body.minor = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`ApiVersion::api_version`](./struct.ApiVersion.html#method.api_version) method for a `GET` operation associated with `ApiVersion`.
#[derive(Debug, Clone)]
pub struct ApiVersionGetBuilder;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ApiVersionGetBuilder {
    type Output = ApiVersion;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/system/api-version".into()
    }
}
