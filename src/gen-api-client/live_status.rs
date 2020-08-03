#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct LiveStatus {
    pub current_score: Option<i64>,
    pub current_test: Option<i64>,
    pub finished: bool,
}

impl LiveStatus {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> LiveStatusBuilder<crate::generics::MissingFinished> {
        LiveStatusBuilder {
            body: Default::default(),
            _finished: core::marker::PhantomData,
        }
    }

    #[inline]
    pub fn get_run_live_status() -> LiveStatusGetBuilder<crate::generics::MissingRunId> {
        LiveStatusGetBuilder {
            inner: Default::default(),
            _param_run_id: core::marker::PhantomData,
        }
    }
}

impl Into<LiveStatus> for LiveStatusBuilder<crate::generics::FinishedExists> {
    fn into(self) -> LiveStatus {
        self.body
    }
}

/// Builder for [`LiveStatus`](./struct.LiveStatus.html) object.
#[derive(Debug, Clone)]
pub struct LiveStatusBuilder<Finished> {
    body: self::LiveStatus,
    _finished: core::marker::PhantomData<Finished>,
}

impl<Finished> LiveStatusBuilder<Finished> {
    #[inline]
    pub fn current_score(mut self, value: impl Into<i64>) -> Self {
        self.body.current_score = Some(value.into());
        self
    }

    #[inline]
    pub fn current_test(mut self, value: impl Into<i64>) -> Self {
        self.body.current_test = Some(value.into());
        self
    }

    #[inline]
    pub fn finished(mut self, value: impl Into<bool>) -> LiveStatusBuilder<crate::generics::FinishedExists> {
        self.body.finished = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`LiveStatus::get_run_live_status`](./struct.LiveStatus.html#method.get_run_live_status) method for a `GET` operation associated with `LiveStatus`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct LiveStatusGetBuilder<RunId> {
    inner: LiveStatusGetBuilderContainer,
    _param_run_id: core::marker::PhantomData<RunId>,
}

#[derive(Debug, Default, Clone)]
struct LiveStatusGetBuilderContainer {
    param_run_id: Option<String>,
}

impl<RunId> LiveStatusGetBuilder<RunId> {
    #[inline]
    pub fn run_id(mut self, value: impl Into<String>) -> LiveStatusGetBuilder<crate::generics::RunIdExists> {
        self.inner.param_run_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for LiveStatusGetBuilder<crate::generics::RunIdExists> {
    type Output = LiveStatus;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/runs/{run_id}/live", run_id=self.inner.param_run_id.as_ref().expect("missing parameter run_id?")).into()
    }
}
