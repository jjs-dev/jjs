
/// Describes updates which will be applied to run
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RunPatch {
    pub binary: Option<String>,
    pub status: Option<Vec<Vec<String>>>,
}

impl RunPatch {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> RunPatchBuilder {
        RunPatchBuilder {
            body: Default::default(),
        }
    }

    /// Modifies existing run
    ///
    /// See `RunPatch` documentation for what can be updated.
    #[inline]
    pub fn patch_run() -> RunPatchPatchBuilder<crate::generics::MissingRunId> {
        RunPatchPatchBuilder {
            inner: Default::default(),
            _param_run_id: core::marker::PhantomData,
        }
    }
}

impl Into<RunPatch> for RunPatchBuilder {
    fn into(self) -> RunPatch {
        self.body
    }
}

impl Into<RunPatch> for RunPatchPatchBuilder<crate::generics::RunIdExists> {
    fn into(self) -> RunPatch {
        self.inner.body
    }
}

/// Builder for [`RunPatch`](./struct.RunPatch.html) object.
#[derive(Debug, Clone)]
pub struct RunPatchBuilder {
    body: self::RunPatch,
}

impl RunPatchBuilder {
    #[inline]
    pub fn binary(mut self, value: impl Into<String>) -> Self {
        self.body.binary = Some(value.into());
        self
    }

    #[inline]
    pub fn status(mut self, value: impl Iterator<Item = impl Iterator<Item = impl Into<String>>>) -> Self {
        self.body.status = Some(value.map(|value| value.map(|value| value.into()).collect::<Vec<_>>().into()).collect::<Vec<_>>().into());
        self
    }
}

/// Builder created by [`RunPatch::patch_run`](./struct.RunPatch.html#method.patch_run) method for a `PATCH` operation associated with `RunPatch`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct RunPatchPatchBuilder<RunId> {
    inner: RunPatchPatchBuilderContainer,
    _param_run_id: core::marker::PhantomData<RunId>,
}

#[derive(Debug, Default, Clone)]
struct RunPatchPatchBuilderContainer {
    body: self::RunPatch,
    param_run_id: Option<String>,
}

impl<RunId> RunPatchPatchBuilder<RunId> {
    #[inline]
    pub fn run_id(mut self, value: impl Into<String>) -> RunPatchPatchBuilder<crate::generics::RunIdExists> {
        self.inner.param_run_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn binary(mut self, value: impl Into<String>) -> Self {
        self.inner.body.binary = Some(value.into());
        self
    }

    #[inline]
    pub fn status(mut self, value: impl Iterator<Item = impl Iterator<Item = impl Into<String>>>) -> Self {
        self.inner.body.status = Some(value.map(|value| value.map(|value| value.into()).collect::<Vec<_>>().into()).collect::<Vec<_>>().into());
        self
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for RunPatchPatchBuilder<crate::generics::RunIdExists> {
    type Output = crate::run::Run;

    const METHOD: http::Method = http::Method::PATCH;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/runs/{run_id}", run_id=self.inner.param_run_id.as_ref().expect("missing parameter run_id?")).into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .json(&self.inner.body))
    }
}
