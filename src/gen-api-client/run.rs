#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Run {
    pub contest_id: String,
    pub id: String,
    pub problem_id: String,
    pub toolchain_id: String,
    pub user_id: String,
}

impl Run {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> RunBuilder<crate::generics::MissingContestId, crate::generics::MissingId, crate::generics::MissingProblemId, crate::generics::MissingToolchainId, crate::generics::MissingUserId> {
        RunBuilder {
            body: Default::default(),
            _contest_id: core::marker::PhantomData,
            _id: core::marker::PhantomData,
            _problem_id: core::marker::PhantomData,
            _toolchain_id: core::marker::PhantomData,
            _user_id: core::marker::PhantomData,
        }
    }

    /// Lists runs
    ///
    /// This operation returns all created runs
    #[inline]
    pub fn list_runs() -> RunGetBuilder {
        RunGetBuilder
    }
}

impl Into<Run> for RunBuilder<crate::generics::ContestIdExists, crate::generics::IdExists, crate::generics::ProblemIdExists, crate::generics::ToolchainIdExists, crate::generics::UserIdExists> {
    fn into(self) -> Run {
        self.body
    }
}

/// Builder for [`Run`](./struct.Run.html) object.
#[derive(Debug, Clone)]
pub struct RunBuilder<ContestId, Id, ProblemId, ToolchainId, UserId> {
    body: self::Run,
    _contest_id: core::marker::PhantomData<ContestId>,
    _id: core::marker::PhantomData<Id>,
    _problem_id: core::marker::PhantomData<ProblemId>,
    _toolchain_id: core::marker::PhantomData<ToolchainId>,
    _user_id: core::marker::PhantomData<UserId>,
}

impl<ContestId, Id, ProblemId, ToolchainId, UserId> RunBuilder<ContestId, Id, ProblemId, ToolchainId, UserId> {
    #[inline]
    pub fn contest_id(mut self, value: impl Into<String>) -> RunBuilder<crate::generics::ContestIdExists, Id, ProblemId, ToolchainId, UserId> {
        self.body.contest_id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn id(mut self, value: impl Into<String>) -> RunBuilder<ContestId, crate::generics::IdExists, ProblemId, ToolchainId, UserId> {
        self.body.id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem_id(mut self, value: impl Into<String>) -> RunBuilder<ContestId, Id, crate::generics::ProblemIdExists, ToolchainId, UserId> {
        self.body.problem_id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn toolchain_id(mut self, value: impl Into<String>) -> RunBuilder<ContestId, Id, ProblemId, crate::generics::ToolchainIdExists, UserId> {
        self.body.toolchain_id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn user_id(mut self, value: impl Into<String>) -> RunBuilder<ContestId, Id, ProblemId, ToolchainId, crate::generics::UserIdExists> {
        self.body.user_id = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`Run::list_runs`](./struct.Run.html#method.list_runs) method for a `GET` operation associated with `Run`.
#[derive(Debug, Clone)]
pub struct RunGetBuilder;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for RunGetBuilder {
    type Output = Vec<Run>;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/runs".into()
    }
}
