use std::sync::Arc;

#[derive(Clone)]
pub struct SecretKey(pub Arc<[u8]>);

impl std::ops::Deref for SecretKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*(self.0)
    }
}
