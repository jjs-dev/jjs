
/// Namespace for operations that cannot be added to any other modules.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Miscellaneous {}

impl Miscellaneous {
    /// Returns if JJS is running in development mode.
    ///
    /// Please note that you don't have to respect this information, but following is recommended:
    /// 1. Display it in each page/view.
    /// 2. Change theme.
    /// 3. On login view, add button "login as root".
    #[inline]
    pub fn is_dev() -> MiscellaneousGetBuilder {
        MiscellaneousGetBuilder
    }
}

/// Builder created by [`Miscellaneous::is_dev`](./struct.Miscellaneous.html#method.is_dev) method for a `GET` operation associated with `Miscellaneous`.
#[derive(Debug, Clone)]
pub struct MiscellaneousGetBuilder;


impl<Client: crate::client::ApiClient + Sync + 'static> crate::client::Sendable<Client> for MiscellaneousGetBuilder {
    type Output = bool;

    const METHOD: http::Method = http::Method::GET;

    fn rel_path(&self) -> std::borrow::Cow<'static, str> {
        "/system/is-dev".into()
    }
}
