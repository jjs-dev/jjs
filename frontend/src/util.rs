pub(crate) enum Env {
    Dev,
    Prod,
}

impl Env {
    pub(crate) fn is_dev(&self) -> bool {
        match self {
            Env::Dev => true,
            Env::Prod => false,
        }
    }
}
