//! Defines entites - types, used throughout all JJS code base
//! Entities are plain dumb structs
pub mod contest;
pub mod toolchain;

use serde::{de::DeserializeOwned, Serialize};
use std::{any::Any, fmt::Debug};
pub(crate) mod seal {
    pub trait Seal {}
}
use seal::Seal;
pub trait Entity: Serialize + DeserializeOwned + Send + Sync + Debug + Any + Seal {
    fn name(&self) -> &str;

    fn postprocess(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn ensure_entity<T: Entity>() {}

    #[test]
    fn test_contest_is_entity() {
        ensure_entity::<contest::Contest>()
    }
    #[test]
    fn test_toolchain_is_entity() {
        ensure_entity::<toolchain::Toolchain>()
    }
}
