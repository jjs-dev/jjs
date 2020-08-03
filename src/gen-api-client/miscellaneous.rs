
/// Namespace for operations that cannot be added to any other modules.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Miscellaneous {}

impl Miscellaneous {
    #[inline]
    pub fn put_problem() -> MiscellaneousPutBuilder<crate::generics::MissingProblemId, crate::generics::MissingProblemAssets, crate::generics::MissingProblemManifest> {
        MiscellaneousPutBuilder {
            inner: Default::default(),
            _param_problem_id: core::marker::PhantomData,
            _param_problem_assets: core::marker::PhantomData,
            _param_problem_manifest: core::marker::PhantomData,
        }
    }

    /// Returns run source as base64-encoded JSON string
    #[inline]
    pub fn get_run_source() -> MiscellaneousGetBuilder1<crate::generics::MissingRunId> {
        MiscellaneousGetBuilder1 {
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
    pub fn is_dev() -> MiscellaneousGetBuilder2 {
        MiscellaneousGetBuilder2
    }
}

/// Builder created by [`Miscellaneous::put_problem`](./struct.Miscellaneous.html#method.put_problem) method for a `PUT` operation associated with `Miscellaneous`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct MiscellaneousPutBuilder<ProblemId, ProblemAssets, ProblemManifest> {
    inner: MiscellaneousPutBuilderContainer,
    _param_problem_id: core::marker::PhantomData<ProblemId>,
    _param_problem_assets: core::marker::PhantomData<ProblemAssets>,
    _param_problem_manifest: core::marker::PhantomData<ProblemManifest>,
}

#[derive(Debug, Default, Clone)]
struct MiscellaneousPutBuilderContainer {
    param_problem_id: Option<String>,
    param_problem_assets: Option<String>,
    param_problem_manifest: Option<String>,
}

impl<ProblemId, ProblemAssets, ProblemManifest> MiscellaneousPutBuilder<ProblemId, ProblemAssets, ProblemManifest> {
    #[inline]
    pub fn problem_id(mut self, value: impl Into<String>) -> MiscellaneousPutBuilder<crate::generics::ProblemIdExists, ProblemAssets, ProblemManifest> {
        self.inner.param_problem_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem_assets(mut self, value: impl Into<String>) -> MiscellaneousPutBuilder<ProblemId, crate::generics::ProblemAssetsExists, ProblemManifest> {
        self.inner.param_problem_assets = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem_manifest(mut self, value: impl Into<String>) -> MiscellaneousPutBuilder<ProblemId, ProblemAssets, crate::generics::ProblemManifestExists> {
        self.inner.param_problem_manifest = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for MiscellaneousPutBuilder<crate::generics::ProblemIdExists, crate::generics::ProblemAssetsExists, crate::generics::ProblemManifestExists> {
    type Output = bool;

    const METHOD: http::Method = http::Method::PUT;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/problems/{problem_id}", problem_id=self.inner.param_problem_id.as_ref().expect("missing parameter problem_id?")).into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .body_bytes({
            let mut ser = url::form_urlencoded::Serializer::new(String::new());
            if let Some(v) = self.inner.param_problem_assets.as_ref() {
                ser.append_pair("problem_assets", &v.to_string());
            }
            if let Some(v) = self.inner.param_problem_manifest.as_ref() {
                ser.append_pair("problem_manifest", &v.to_string());
            }
            ser.finish().into_bytes()
        })
        .header(http::header::CONTENT_TYPE.as_str(), "application/x-www-form-urlencoded"))
    }
}

/// Builder created by [`Miscellaneous::get_run_source`](./struct.Miscellaneous.html#method.get_run_source) method for a `GET` operation associated with `Miscellaneous`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct MiscellaneousGetBuilder1<RunId> {
    inner: MiscellaneousGetBuilder1Container,
    _param_run_id: core::marker::PhantomData<RunId>,
}

#[derive(Debug, Default, Clone)]
struct MiscellaneousGetBuilder1Container {
    param_run_id: Option<String>,
}

impl<RunId> MiscellaneousGetBuilder1<RunId> {
    #[inline]
    pub fn run_id(mut self, value: impl Into<String>) -> MiscellaneousGetBuilder1<crate::generics::RunIdExists> {
        self.inner.param_run_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for MiscellaneousGetBuilder1<crate::generics::RunIdExists> {
    type Output = String;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/runs/{run_id}/source", run_id=self.inner.param_run_id.as_ref().expect("missing parameter run_id?")).into()
    }
}

/// Builder created by [`Miscellaneous::is_dev`](./struct.Miscellaneous.html#method.is_dev) method for a `GET` operation associated with `Miscellaneous`.
#[derive(Debug, Clone)]
pub struct MiscellaneousGetBuilder2;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for MiscellaneousGetBuilder2 {
    type Output = bool;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/system/is-dev".into()
    }
}
