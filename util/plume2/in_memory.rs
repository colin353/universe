use std::sync::Arc;

use crate::iter::StreamingIterator;
use crate::{globals, Error};
use plume_bus::*;

pub struct InMemoryCollection {
    pub data: Arc<dyn InMemoryCollectionWrapper>,
}

impl InMemoryCollection {
    pub fn from_vec<T>(data: Vec<T>) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            data: Arc::new(InMemoryCollectionUnderlying {
                data: Arc::new(data),
            }),
        }
    }
    pub fn from_table<T>(data: Vec<(String, T)>) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            data: Arc::new(InMemoryTableUnderlying {
                data: Arc::new(data),
            }),
        }
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn get_vec<T: Send + Sync + 'static>(&self) -> Result<Arc<Vec<T>>, Error> {
        let pcoll: Arc<dyn InMemoryCollectionWrapper> = self.data.clone();
        let data: &InMemoryCollectionUnderlying<T> = pcoll.downcast_ref()?;
        Ok(data.data.clone())
    }
}

pub trait InMemoryCollectionWrapper: Send + Sync {
    fn len(&self) -> usize;
    fn keyranges(&self, target_shards: usize) -> Vec<String>;
}

impl dyn InMemoryCollectionWrapper {
    pub fn downcast_ref<T: InMemoryCollectionWrapper>(&self) -> Result<&T, Error> {
        // TODO: I don't know how to actually check if the types match. Need to do something like
        // verify the type ID. Basically if they don't match the program will do some undefined
        // behaviour so we should actually check and panic.
        unsafe { Ok(self.downcast_ref_unchecked()) }
    }

    pub unsafe fn downcast_ref_unchecked<T: InMemoryCollectionWrapper>(&self) -> &T {
        &*(self as *const Self as *const T)
    }
}

pub struct InMemoryCollectionUnderlying<T> {
    data: Arc<Vec<T>>,
}

pub struct InMemoryTableUnderlying<T> {
    data: Arc<Vec<(String, T)>>,
}

impl<T> InMemoryCollectionWrapper for InMemoryCollectionUnderlying<T>
where
    T: Send + Sync + 'static,
{
    fn len(&self) -> usize {
        self.data.len()
    }
    fn keyranges(&self, _: usize) -> Vec<String> {
        Vec::new()
    }
}

impl<T> InMemoryCollectionWrapper for InMemoryTableUnderlying<T>
where
    T: Send + Sync + 'static,
{
    fn len(&self) -> usize {
        self.data.len()
    }
    fn keyranges(&self, target_shards: usize) -> Vec<String> {
        if self.data.len() == 0 {
            return Vec::new();
        }
        let mut keyranges: Vec<_> = (1..(target_shards))
            .map(|i| self.data[i * self.data.len() / target_shards].0.to_string())
            .filter(|k| !k.is_empty())
            .collect();
        keyranges.dedup();
        keyranges
    }
}

struct InMemorySourceIterator<T> {
    data: Arc<Vec<T>>,
    index: usize,
    end_index: usize,
}

impl<T> StreamingIterator for InMemorySourceIterator<T> {
    type Item = T;

    fn advance(&mut self) {
        self.index += 1;
    }

    fn get(&self) -> Option<&T> {
        if self.index >= self.end_index {
            return None;
        }

        Some(&self.data[self.index])
    }
}

pub fn mem_source<T: Send + Sync + 'static>(
    spec: &CollectionSpec,
) -> Box<dyn StreamingIterator<Item = T>> {
    crate::iter::chain(
        spec.memory_ids
            .iter()
            .map(|id| {
                let guard = globals::IN_MEMORY_DATASETS
                    .read()
                    .expect("failed to acquire readlock");
                let dataset = guard.get(id).expect("failed to look up in memory data");
                let pcoll: Arc<dyn InMemoryCollectionWrapper> = dataset.data.clone();
                let data: &InMemoryCollectionUnderlying<T> =
                    pcoll.downcast_ref().expect("failed to downcast!");
                let out: Box<dyn StreamingIterator<Item = T>> = Box::new(InMemorySourceIterator {
                    data: data.data.clone(),
                    index: spec.starting_index as usize,
                    end_index: spec.ending_index as usize,
                });
                out
            })
            .collect(),
    )
}
