#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Toolchain {
    pub description: String,
    pub id: String,
    pub image: String,
}

impl Toolchain {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> ToolchainBuilder<crate::generics::MissingDescription, crate::generics::MissingId, crate::generics::MissingImage> {
        ToolchainBuilder {
            body: Default::default(),
            _description: core::marker::PhantomData,
            _id: core::marker::PhantomData,
            _image: core::marker::PhantomData,
        }
    }

    #[inline]
    pub fn list_toolchains() -> ToolchainGetBuilder {
        ToolchainGetBuilder
    }

    #[inline]
    pub fn put_toolchain() -> ToolchainPutBuilder<crate::generics::MissingDescription, crate::generics::MissingId, crate::generics::MissingImage> {
        ToolchainPutBuilder {
            body: Default::default(),
            _description: core::marker::PhantomData,
            _id: core::marker::PhantomData,
            _image: core::marker::PhantomData,
        }
    }

    #[inline]
    pub fn get_toolchain() -> ToolchainGetBuilder1<crate::generics::MissingToolchainId> {
        ToolchainGetBuilder1 {
            inner: Default::default(),
            _param_toolchain_id: core::marker::PhantomData,
        }
    }
}

impl Into<Toolchain> for ToolchainBuilder<crate::generics::DescriptionExists, crate::generics::IdExists, crate::generics::ImageExists> {
    fn into(self) -> Toolchain {
        self.body
    }
}

impl Into<Toolchain> for ToolchainPutBuilder<crate::generics::DescriptionExists, crate::generics::IdExists, crate::generics::ImageExists> {
    fn into(self) -> Toolchain {
        self.body
    }
}

/// Builder for [`Toolchain`](./struct.Toolchain.html) object.
#[derive(Debug, Clone)]
pub struct ToolchainBuilder<Description, Id, Image> {
    body: self::Toolchain,
    _description: core::marker::PhantomData<Description>,
    _id: core::marker::PhantomData<Id>,
    _image: core::marker::PhantomData<Image>,
}

impl<Description, Id, Image> ToolchainBuilder<Description, Id, Image> {
    #[inline]
    pub fn description(mut self, value: impl Into<String>) -> ToolchainBuilder<crate::generics::DescriptionExists, Id, Image> {
        self.body.description = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn id(mut self, value: impl Into<String>) -> ToolchainBuilder<Description, crate::generics::IdExists, Image> {
        self.body.id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn image(mut self, value: impl Into<String>) -> ToolchainBuilder<Description, Id, crate::generics::ImageExists> {
        self.body.image = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`Toolchain::list_toolchains`](./struct.Toolchain.html#method.list_toolchains) method for a `GET` operation associated with `Toolchain`.
#[derive(Debug, Clone)]
pub struct ToolchainGetBuilder;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ToolchainGetBuilder {
    type Output = Vec<Toolchain>;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/toolchains".into()
    }
}

/// Builder created by [`Toolchain::put_toolchain`](./struct.Toolchain.html#method.put_toolchain) method for a `PUT` operation associated with `Toolchain`.
#[derive(Debug, Clone)]
pub struct ToolchainPutBuilder<Description, Id, Image> {
    body: self::Toolchain,
    _description: core::marker::PhantomData<Description>,
    _id: core::marker::PhantomData<Id>,
    _image: core::marker::PhantomData<Image>,
}

impl<Description, Id, Image> ToolchainPutBuilder<Description, Id, Image> {
    #[inline]
    pub fn description(mut self, value: impl Into<String>) -> ToolchainPutBuilder<crate::generics::DescriptionExists, Id, Image> {
        self.body.description = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn id(mut self, value: impl Into<String>) -> ToolchainPutBuilder<Description, crate::generics::IdExists, Image> {
        self.body.id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn image(mut self, value: impl Into<String>) -> ToolchainPutBuilder<Description, Id, crate::generics::ImageExists> {
        self.body.image = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ToolchainPutBuilder<crate::generics::DescriptionExists, crate::generics::IdExists, crate::generics::ImageExists> {
    type Output = crate::toolchain::Toolchain;

    const METHOD: http::Method = http::Method::PUT;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/toolchains".into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .json(&self.body))
    }
}

/// Builder created by [`Toolchain::get_toolchain`](./struct.Toolchain.html#method.get_toolchain) method for a `GET` operation associated with `Toolchain`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ToolchainGetBuilder1<ToolchainId> {
    inner: ToolchainGetBuilder1Container,
    _param_toolchain_id: core::marker::PhantomData<ToolchainId>,
}

#[derive(Debug, Default, Clone)]
struct ToolchainGetBuilder1Container {
    param_toolchain_id: Option<String>,
}

impl<ToolchainId> ToolchainGetBuilder1<ToolchainId> {
    #[inline]
    pub fn toolchain_id(mut self, value: impl Into<String>) -> ToolchainGetBuilder1<crate::generics::ToolchainIdExists> {
        self.inner.param_toolchain_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ToolchainGetBuilder1<crate::generics::ToolchainIdExists> {
    type Output = Toolchain;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/toolchains/{toolchain_id}", toolchain_id=self.inner.param_toolchain_id.as_ref().expect("missing parameter toolchain_id?")).into()
    }
}
