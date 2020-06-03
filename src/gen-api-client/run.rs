#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Run {
    pub contest_name: String,
    pub id: String,
    pub problem_name: String,
    pub status: Option<std::collections::BTreeMap<String, String>>,
    pub toolchain_name: String,
    pub user_id: String,
}

impl Run {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> RunBuilder<crate::generics::MissingContestName, crate::generics::MissingId, crate::generics::MissingProblemName, crate::generics::MissingToolchainName, crate::generics::MissingUserId> {
        RunBuilder {
            body: Default::default(),
            _contest_name: core::marker::PhantomData,
            _id: core::marker::PhantomData,
            _problem_name: core::marker::PhantomData,
            _toolchain_name: core::marker::PhantomData,
            _user_id: core::marker::PhantomData,
        }
    }

    /// Returns runs that should be judged
    ///
    /// At most `limit` runs will be returned
    ///
    /// These runs are immediately locked, to prevent resource wasting.
    /// However, this is not safe distributed lock: on timeout lock will
    /// be released. It means, that in some rare situations same run can be judged
    /// several times. All judgings except one will be ignored.
    #[inline]
    pub fn pop_run_from_queue() -> RunPostBuilder<crate::generics::MissingLimit> {
        RunPostBuilder {
            inner: Default::default(),
            _param_limit: core::marker::PhantomData,
        }
    }

    /// Lists runs
    ///
    /// This operation returns all created runs
    #[inline]
    pub fn list_runs() -> RunGetBuilder1 {
        RunGetBuilder1
    }

    /// Loads run by id
    #[inline]
    pub fn get_run() -> RunGetBuilder2<crate::generics::MissingRunId> {
        RunGetBuilder2 {
            inner: Default::default(),
            _param_run_id: core::marker::PhantomData,
        }
    }
}

impl Into<Run> for RunBuilder<crate::generics::ContestNameExists, crate::generics::IdExists, crate::generics::ProblemNameExists, crate::generics::ToolchainNameExists, crate::generics::UserIdExists> {
    fn into(self) -> Run {
        self.body
    }
}

/// Builder for [`Run`](./struct.Run.html) object.
#[derive(Debug, Clone)]
pub struct RunBuilder<ContestName, Id, ProblemName, ToolchainName, UserId> {
    body: self::Run,
    _contest_name: core::marker::PhantomData<ContestName>,
    _id: core::marker::PhantomData<Id>,
    _problem_name: core::marker::PhantomData<ProblemName>,
    _toolchain_name: core::marker::PhantomData<ToolchainName>,
    _user_id: core::marker::PhantomData<UserId>,
}

impl<ContestName, Id, ProblemName, ToolchainName, UserId> RunBuilder<ContestName, Id, ProblemName, ToolchainName, UserId> {
    #[inline]
    pub fn contest_name(mut self, value: impl Into<String>) -> RunBuilder<crate::generics::ContestNameExists, Id, ProblemName, ToolchainName, UserId> {
        self.body.contest_name = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn id(mut self, value: impl Into<String>) -> RunBuilder<ContestName, crate::generics::IdExists, ProblemName, ToolchainName, UserId> {
        self.body.id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem_name(mut self, value: impl Into<String>) -> RunBuilder<ContestName, Id, crate::generics::ProblemNameExists, ToolchainName, UserId> {
        self.body.problem_name = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn status(mut self, value: impl Iterator<Item = (String, impl Into<String>)>) -> Self {
        self.body.status = Some(value.map(|(key, value)| (key, value.into())).collect::<std::collections::BTreeMap<_, _>>().into());
        self
    }

    #[inline]
    pub fn toolchain_name(mut self, value: impl Into<String>) -> RunBuilder<ContestName, Id, ProblemName, crate::generics::ToolchainNameExists, UserId> {
        self.body.toolchain_name = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn user_id(mut self, value: impl Into<String>) -> RunBuilder<ContestName, Id, ProblemName, ToolchainName, crate::generics::UserIdExists> {
        self.body.user_id = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`Run::pop_run_from_queue`](./struct.Run.html#method.pop_run_from_queue) method for a `POST` operation associated with `Run`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct RunPostBuilder<Limit> {
    inner: RunPostBuilderContainer,
    _param_limit: core::marker::PhantomData<Limit>,
}

#[derive(Debug, Default, Clone)]
struct RunPostBuilderContainer {
    param_limit: Option<i64>,
}

impl<Limit> RunPostBuilder<Limit> {
    #[inline]
    pub fn limit(mut self, value: impl Into<i64>) -> RunPostBuilder<crate::generics::LimitExists> {
        self.inner.param_limit = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for RunPostBuilder<crate::generics::LimitExists> {
    type Output = Vec<Run>;

    const METHOD: http::Method = http::Method::POST;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/queue".into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .query(&[
            ("limit", self.inner.param_limit.as_ref().map(std::string::ToString::to_string))
        ]))
    }
}

/// Builder created by [`Run::list_runs`](./struct.Run.html#method.list_runs) method for a `GET` operation associated with `Run`.
#[derive(Debug, Clone)]
pub struct RunGetBuilder1;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for RunGetBuilder1 {
    type Output = Vec<Run>;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/runs".into()
    }
}

/// Builder created by [`Run::get_run`](./struct.Run.html#method.get_run) method for a `GET` operation associated with `Run`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct RunGetBuilder2<RunId> {
    inner: RunGetBuilder2Container,
    _param_run_id: core::marker::PhantomData<RunId>,
}

#[derive(Debug, Default, Clone)]
struct RunGetBuilder2Container {
    param_run_id: Option<String>,
}

impl<RunId> RunGetBuilder2<RunId> {
    #[inline]
    pub fn run_id(mut self, value: impl Into<String>) -> RunGetBuilder2<crate::generics::RunIdExists> {
        self.inner.param_run_id = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for RunGetBuilder2<crate::generics::RunIdExists> {
    type Output = Run;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/runs/{run_id}", run_id=self.inner.param_run_id.as_ref().expect("missing parameter run_id?")).into()
    }
}
