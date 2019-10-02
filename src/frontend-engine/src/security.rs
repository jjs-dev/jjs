mod access_ck;
mod token;
mod token_mgr;

pub(crate) use access_ck::AccessChecker;
use std::sync::Arc;
pub(crate) use token::Token;
pub use token_mgr::{TokenMgr, TokenMgrError};

#[derive(Clone)]
pub struct SecretKey(pub Arc<[u8]>);

impl std::ops::Deref for SecretKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*(self.0)
    }
}
