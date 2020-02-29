mod builder;

pub use builder::LoaderBuilder;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
};

use crate::entities::Entity;

pub struct Loader {
    entities: HashMap<TypeId, HashMap<String, Box<dyn Any + Send + Sync>>>,
}

impl Loader {
    pub fn list<T: Entity>(&self) -> impl Iterator<Item = &T> {
        let key = TypeId::of::<T>();
        match self.entities.get(&key) {
            Some(map) => {
                let iter = map
                    .values()
                    .map(|any_box| any_box.downcast_ref().expect("corrupted typemap in Loader"));
                either::Either::Left(iter)
            }
            None => either::Either::Right(std::iter::empty()),
        }
    }

    pub fn find<T: Entity>(&self, name: &str) -> Option<&T> {
        let key = TypeId::of::<T>();
        self.entities
            .get(&key)
            .and_then(|map| map.get(name))
            .map(|any_box| any_box.downcast_ref().expect("corrupted typemap in Loader"))
    }
}

impl Loader {
    pub fn load_from_data_dir(dir: &std::path::Path) -> anyhow::Result<Loader> {
        let mut builder = LoaderBuilder::new();
        builder.load_from_data_dir(dir)?;
        Ok(builder.into_inner())
    }
}
