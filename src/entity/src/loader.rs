mod builder;

pub use builder::LoaderBuilder;

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    sync::Arc,
};

use crate::entities::Entity;

#[derive(Debug)]
pub(crate) struct EntitiesData {
    entities: HashMap<TypeId, HashMap<String, Box<dyn Any + Send + Sync>>>,
}

#[derive(Clone, Debug)]
pub struct Loader(Arc<EntitiesData>);

impl Loader {
    fn from_inner(data: Arc<EntitiesData>) -> Loader {
        Self(data)
    }

    pub fn list<T: Entity>(&self) -> impl Iterator<Item = &T> {
        let key = TypeId::of::<T>();
        match self.0.entities.get(&key) {
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
        self.0
            .entities
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
