#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub name: String,
    pub rel_name: String,
    pub title: String,
}

impl Problem {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> ProblemBuilder<crate::generics::MissingName, crate::generics::MissingRelName, crate::generics::MissingTitle> {
        ProblemBuilder {
            body: Default::default(),
            _name: core::marker::PhantomData,
            _rel_name: core::marker::PhantomData,
            _title: core::marker::PhantomData,
        }
    }

    #[inline]
    pub fn list_contest_problems() -> ProblemGetBuilder<crate::generics::MissingContestName> {
        ProblemGetBuilder {
            inner: Default::default(),
            _param_contest_name: core::marker::PhantomData,
        }
    }
}

impl Into<Problem> for ProblemBuilder<crate::generics::NameExists, crate::generics::RelNameExists, crate::generics::TitleExists> {
    fn into(self) -> Problem {
        self.body
    }
}

/// Builder for [`Problem`](./struct.Problem.html) object.
#[derive(Debug, Clone)]
pub struct ProblemBuilder<Name, RelName, Title> {
    body: self::Problem,
    _name: core::marker::PhantomData<Name>,
    _rel_name: core::marker::PhantomData<RelName>,
    _title: core::marker::PhantomData<Title>,
}

impl<Name, RelName, Title> ProblemBuilder<Name, RelName, Title> {
    #[inline]
    pub fn name(mut self, value: impl Into<String>) -> ProblemBuilder<crate::generics::NameExists, RelName, Title> {
        self.body.name = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn rel_name(mut self, value: impl Into<String>) -> ProblemBuilder<Name, crate::generics::RelNameExists, Title> {
        self.body.rel_name = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn title(mut self, value: impl Into<String>) -> ProblemBuilder<Name, RelName, crate::generics::TitleExists> {
        self.body.title = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`Problem::list_contest_problems`](./struct.Problem.html#method.list_contest_problems) method for a `GET` operation associated with `Problem`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ProblemGetBuilder<ContestName> {
    inner: ProblemGetBuilderContainer,
    _param_contest_name: core::marker::PhantomData<ContestName>,
}

#[derive(Debug, Default, Clone)]
struct ProblemGetBuilderContainer {
    param_contest_name: Option<String>,
}

impl<ContestName> ProblemGetBuilder<ContestName> {
    #[inline]
    pub fn contest_name(mut self, value: impl Into<String>) -> ProblemGetBuilder<crate::generics::ContestNameExists> {
        self.inner.param_contest_name = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ProblemGetBuilder<crate::generics::ContestNameExists> {
    type Output = Vec<Problem>;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/contests/{contest_name}/problems", contest_name=self.inner.param_contest_name.as_ref().expect("missing parameter contest_name?")).into()
    }
}
