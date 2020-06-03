
/// Namespace for operations that cannot be added to any other modules.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Miscellaneous {}

impl Miscellaneous {
    /// Returns run source as base64-encoded JSON string
    #[inline]
    pub fn get_run_source() -> MiscellaneousGetBuilder<crate::generics::MissingRunId> {
        MiscellaneousGetBuilder {
            inner: Default::default(),
            _param_run_id: core::marker::PhantomData,
        }
    }

    /// Returns if JJS is running in development mode
    ///
    /// Please note that you don't have to respect this information, but following is recommended:
    /// 1. Display it in each page/view.
    /// 2. Change theme.
    /// 3. On login view, add button "login as root".
    #[inline]
    pub fn is_dev() -> MiscellaneousGetBuilder1 {
        MiscellaneousGetBuilder1
    }
}

/// Builder created by [`Miscellaneous::get_run_source`](./struct.Miscellaneous.html#method.get_run_source) method for a `GET` operation associated with `Miscellaneous`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct MiscellaneousGetBuilder<RunId> {
    inner: MiscellaneousGetBuilderContainer,
    _param_run_id: core::marker::PhantomData<RunId>,
}

#[derive(Debug, Default, Clone)]
struct MiscellaneousGetBuilderContainer {
    param_run_id: Option<String>,
}

impl<RunId> MiscellaneousGetBuilder<RunId> {
    #[inline]
    pub fn run_id(mut self, value: impl Into<String>) -> MiscellaneousGetBuilder<crate::generics::RunIdExists> {
        self.inner.param_run_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for MiscellaneousGetBuilder<crate::generics::RunIdExists> {
    type Output = String;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/runs/{run_id}/source", run_id=self.inner.param_run_id.as_ref().expect("missing parameter run_id?")).into()
    }
}

/// Builder created by [`Miscellaneous::is_dev`](./struct.Miscellaneous.html#method.is_dev) method for a `GET` operation associated with `Miscellaneous`.
#[derive(Debug, Clone)]
pub struct MiscellaneousGetBuilder1;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for MiscellaneousGetBuilder1 {
    type Output = bool;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/system/is-dev".into()
    }
}
