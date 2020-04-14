use std::rc::Rc;

#[derive(Clone)]
pub struct SecretKey(pub Rc<[u8]>);

impl std::ops::Deref for SecretKey {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &*(self.0)
    }
}
