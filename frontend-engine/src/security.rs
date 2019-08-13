mod token;

use std::sync::Arc;
pub use token::{Token, TokenFromRequestError};

#[derive(Clone)]
pub struct SecretKey(pub Arc<[u8]>);
