#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct RunSubmitSimpleParams {
    pub code: String,
    pub contest: String,
    pub problem: String,
    pub toolchain: String,
}

impl RunSubmitSimpleParams {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> RunSubmitSimpleParamsBuilder<crate::generics::MissingCode, crate::generics::MissingContest, crate::generics::MissingProblem, crate::generics::MissingToolchain> {
        RunSubmitSimpleParamsBuilder {
            body: Default::default(),
            _code: core::marker::PhantomData,
            _contest: core::marker::PhantomData,
            _problem: core::marker::PhantomData,
            _toolchain: core::marker::PhantomData,
        }
    }

    /// Submits new run
    ///
    /// This operation creates new run, with given source code, and queues it for
    /// judging. Created run will be returned. All fields against `id` will match
    /// fields of request body; `id` will be real id of this run.
    #[inline]
    pub fn submit_run() -> RunSubmitSimpleParamsPostBuilder<crate::generics::MissingCode, crate::generics::MissingContest, crate::generics::MissingProblem, crate::generics::MissingToolchain> {
        RunSubmitSimpleParamsPostBuilder {
            body: Default::default(),
            _code: core::marker::PhantomData,
            _contest: core::marker::PhantomData,
            _problem: core::marker::PhantomData,
            _toolchain: core::marker::PhantomData,
        }
    }
}

impl Into<RunSubmitSimpleParams> for RunSubmitSimpleParamsBuilder<crate::generics::CodeExists, crate::generics::ContestExists, crate::generics::ProblemExists, crate::generics::ToolchainExists> {
    fn into(self) -> RunSubmitSimpleParams {
        self.body
    }
}

impl Into<RunSubmitSimpleParams> for RunSubmitSimpleParamsPostBuilder<crate::generics::CodeExists, crate::generics::ContestExists, crate::generics::ProblemExists, crate::generics::ToolchainExists> {
    fn into(self) -> RunSubmitSimpleParams {
        self.body
    }
}

/// Builder for [`RunSubmitSimpleParams`](./struct.RunSubmitSimpleParams.html) object.
#[derive(Debug, Clone)]
pub struct RunSubmitSimpleParamsBuilder<Code, Contest, Problem, Toolchain> {
    body: self::RunSubmitSimpleParams,
    _code: core::marker::PhantomData<Code>,
    _contest: core::marker::PhantomData<Contest>,
    _problem: core::marker::PhantomData<Problem>,
    _toolchain: core::marker::PhantomData<Toolchain>,
}

impl<Code, Contest, Problem, Toolchain> RunSubmitSimpleParamsBuilder<Code, Contest, Problem, Toolchain> {
    #[inline]
    pub fn code(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsBuilder<crate::generics::CodeExists, Contest, Problem, Toolchain> {
        self.body.code = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn contest(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsBuilder<Code, crate::generics::ContestExists, Problem, Toolchain> {
        self.body.contest = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsBuilder<Code, Contest, crate::generics::ProblemExists, Toolchain> {
        self.body.problem = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn toolchain(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsBuilder<Code, Contest, Problem, crate::generics::ToolchainExists> {
        self.body.toolchain = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`RunSubmitSimpleParams::submit_run`](./struct.RunSubmitSimpleParams.html#method.submit_run) method for a `POST` operation associated with `RunSubmitSimpleParams`.
#[derive(Debug, Clone)]
pub struct RunSubmitSimpleParamsPostBuilder<Code, Contest, Problem, Toolchain> {
    body: self::RunSubmitSimpleParams,
    _code: core::marker::PhantomData<Code>,
    _contest: core::marker::PhantomData<Contest>,
    _problem: core::marker::PhantomData<Problem>,
    _toolchain: core::marker::PhantomData<Toolchain>,
}

impl<Code, Contest, Problem, Toolchain> RunSubmitSimpleParamsPostBuilder<Code, Contest, Problem, Toolchain> {
    #[inline]
    pub fn code(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsPostBuilder<crate::generics::CodeExists, Contest, Problem, Toolchain> {
        self.body.code = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn contest(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsPostBuilder<Code, crate::generics::ContestExists, Problem, Toolchain> {
        self.body.contest = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn problem(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsPostBuilder<Code, Contest, crate::generics::ProblemExists, Toolchain> {
        self.body.problem = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn toolchain(mut self, value: impl Into<String>) -> RunSubmitSimpleParamsPostBuilder<Code, Contest, Problem, crate::generics::ToolchainExists> {
        self.body.toolchain = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for RunSubmitSimpleParamsPostBuilder<crate::generics::CodeExists, crate::generics::ContestExists, crate::generics::ProblemExists, crate::generics::ToolchainExists> {
    type Output = crate::run::Run;

    const METHOD: http::Method = http::Method::POST;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/runs".into()
    }

    fn modify(&self, req: Client::Request) -> Result<Client::Request, crate::client::ApiError<Client::Response>> {
        use crate::client::Request;
        Ok(req
        .json(&self.body))
    }
}
