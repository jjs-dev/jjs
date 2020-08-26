#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Contest {
    pub id: String,
    pub title: String,
}

impl Contest {
    /// Create a builder for this object.
    #[inline]
    pub fn builder() -> ContestBuilder<crate::generics::MissingId, crate::generics::MissingTitle> {
        ContestBuilder {
            body: Default::default(),
            _id: core::marker::PhantomData,
            _title: core::marker::PhantomData,
        }
    }

    #[inline]
    pub fn list_contests() -> ContestGetBuilder {
        ContestGetBuilder
    }

    #[inline]
    pub fn get_contest() -> ContestGetBuilder1<crate::generics::MissingContestName> {
        ContestGetBuilder1 {
            inner: Default::default(),
            _param_contest_name: core::marker::PhantomData,
        }
    }
}

impl Into<Contest> for ContestBuilder<crate::generics::IdExists, crate::generics::TitleExists> {
    fn into(self) -> Contest {
        self.body
    }
}

/// Builder for [`Contest`](./struct.Contest.html) object.
#[derive(Debug, Clone)]
pub struct ContestBuilder<Id, Title> {
    body: self::Contest,
    _id: core::marker::PhantomData<Id>,
    _title: core::marker::PhantomData<Title>,
}

impl<Id, Title> ContestBuilder<Id, Title> {
    #[inline]
    pub fn id(mut self, value: impl Into<String>) -> ContestBuilder<crate::generics::IdExists, Title> {
        self.body.id = value.into();
        unsafe { std::mem::transmute(self) }
    }

    #[inline]
    pub fn title(mut self, value: impl Into<String>) -> ContestBuilder<Id, crate::generics::TitleExists> {
        self.body.title = value.into();
        unsafe { std::mem::transmute(self) }
    }
}

/// Builder created by [`Contest::list_contests`](./struct.Contest.html#method.list_contests) method for a `GET` operation associated with `Contest`.
#[derive(Debug, Clone)]
pub struct ContestGetBuilder;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ContestGetBuilder {
    type Output = Vec<Contest>;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/contests".into()
    }
}

/// Builder created by [`Contest::get_contest`](./struct.Contest.html#method.get_contest) method for a `GET` operation associated with `Contest`.
#[repr(transparent)]
#[derive(Debug, Clone)]
pub struct ContestGetBuilder1<ContestName> {
    inner: ContestGetBuilder1Container,
    _param_contest_name: core::marker::PhantomData<ContestName>,
}

#[derive(Debug, Default, Clone)]
struct ContestGetBuilder1Container {
    param_contest_name: Option<String>,
}

impl<ContestName> ContestGetBuilder1<ContestName> {
    #[inline]
    pub fn contest_name(mut self, value: impl Into<String>) -> ContestGetBuilder1<crate::generics::ContestNameExists> {
        self.inner.param_contest_name = Some(value.into());
        unsafe { std::mem::transmute(self) }
    }
}

impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for ContestGetBuilder1<crate::generics::ContestNameExists> {
    type Output = Contest;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        format!("/contests/{contest_name}", contest_name=self.inner.param_contest_name.as_ref().expect("missing parameter contest_name?")).into()
    }
}
