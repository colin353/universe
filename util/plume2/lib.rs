#![feature(min_specialization)]

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

mod in_memory;
mod iter;
mod map_fn;
mod sink;

use in_memory::{InMemoryCollection, InMemoryCollectionUnderlying, InMemoryCollectionWrapper};
use map_fn::MapFnWrapper;

pub use map_fn::MapFn;

use plume_bus::*;
#[macro_use]
extern crate lazy_static;

#[allow(deprecated)]
mod globals {
    use super::*;
    lazy_static! {
        pub static ref COLLECTION_REGISTRY: RwLock<HashMap<u64, CollectionSpec>> =
            RwLock::new(HashMap::new());
        pub static ref PFN_REGISTRY: RwLock<HashMap<u64, Arc<dyn PFn>>> =
            RwLock::new(HashMap::new());
        pub static ref IN_MEMORY_DATASETS: RwLock<HashMap<u64, InMemoryCollection>> =
            RwLock::new(HashMap::new());
    }
}

#[derive(Debug)]
pub enum Error {
    NotResolved(String),
    WrongFormat(String),
}

pub fn get_collection(id: u64) -> CollectionSpec {
    globals::COLLECTION_REGISTRY
        .read()
        .expect("failed to acquire read lock")
        .get(&id)
        .expect("collection missing")
        .to_owned()
}

pub fn get_pfn(id: u64) -> Arc<dyn PFn> {
    globals::PFN_REGISTRY
        .read()
        .expect("failed to acquire read lock")
        .get(&id)
        .expect("pfn missing")
        .clone()
}

pub trait PFn: Send + Sync + 'static {
    fn plan(&self) -> (Plan, Vec<Plan>);
    fn execute(&self, plan: &Plan);
    fn name(&self) -> String;
    fn init(&self, _: &Plan) {}
}

pub trait EmitFn<T> {
    fn emit(&mut self, value: T);
    fn finish(self: Box<Self>);
}

static LAST_ID: AtomicU64 = AtomicU64::new(1);
fn reserve_id() -> u64 {
    LAST_ID.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
}

pub struct Collection<T> {
    id: u64,
    _marker: std::marker::PhantomData<T>,
}

impl<T> Collection<T>
where
    T: Send + Sync + 'static,
{
    pub fn new_with_changes<F: FnOnce(&mut CollectionSpec)>(f: F) -> Self {
        let id = reserve_id();
        let mut spec = CollectionSpec::new();
        spec.id = id;
        spec.typename = std::any::type_name::<T>()
            .split("::")
            .last()
            .unwrap()
            .to_string();

        f(&mut spec);

        globals::COLLECTION_REGISTRY
            .write()
            .unwrap()
            .insert(id, spec.clone());

        Self {
            id,
            _marker: std::marker::PhantomData {},
        }
    }

    pub fn dependency(&self) -> Option<Arc<dyn PFn>> {
        let dep_id = get_collection(self.id).dependency;
        if dep_id == 0 {
            return None;
        }
        Some(get_pfn(dep_id))
    }

    pub fn new() -> Self {
        Self::new_with_changes(|_| {})
    }

    pub fn plan(&self) -> Vec<Plan> {
        if let Some(pfn) = self.dependency() {
            let (mut s, mut deps) = pfn.plan();
            s.outputs.push(get_collection(self.id));
            deps.push(s);
            return deps;
        }

        Vec::new()
    }

    pub fn from_vec(data: Vec<T>) -> Self {
        Self::from_vecs(vec![data])
    }

    pub fn from_vecs(datas: Vec<Vec<T>>) -> Self {
        let mut memory_ids = Vec::new();
        for data in datas {
            let memory_id = reserve_id();
            memory_ids.push(memory_id);

            globals::IN_MEMORY_DATASETS
                .write()
                .unwrap()
                .insert(memory_id, InMemoryCollection::from_vec(data));
        }

        Self::new_with_changes(|c| {
            c.format = DataFormat::InMemory;
            c.resolved = true;
            c.memory_ids = memory_ids;
        })
    }

    pub fn from_recordio(path: &str) -> Self {
        Self::new_with_changes(|c| {
            c.format = DataFormat::RecordIO;
            c.resolved = true;
            c.filenames.push(path.to_string());
        })
    }

    pub fn map<O, M>(&self, f: M) -> Collection<O>
    where
        M: MapFn<Input = T, Output = O> + 'static,
        O: Send + Sync + 'static,
    {
        let id = reserve_id();
        let out = Collection::new_with_changes(|c| {
            c.dependency = id;
        });
        let pfn = Arc::new(MapFnWrapper::new(self.id, Box::new(f)));
        globals::PFN_REGISTRY.write().unwrap().insert(id, pfn);
        out
    }

    pub fn write_to_vec(&self) {
        let mut r = globals::COLLECTION_REGISTRY.write().unwrap();
        let c = r.get_mut(&self.id).expect("couldn't find collection");

        assert_eq!(
            c.format,
            DataFormat::Unknown,
            "this collection already has an output format!"
        );

        c.format = DataFormat::InMemory;
        c.target_memory_shards = 1;
    }

    pub fn as_vec(&self) -> Result<Arc<Vec<T>>, Error> {
        let c = get_collection(self.id);
        if !c.resolved {
            return Err(Error::NotResolved(
                "collection is not yet resolved".to_string(),
            ));
        }

        if c.format != DataFormat::InMemory {
            return Err(Error::WrongFormat("collection is in memory".to_string()));
        }

        if !c.memory_ids.len() != 1 {
            return Err(Error::WrongFormat(
                "unsupported for sharded outputs".to_string(),
            ));
        }

        let data = globals::IN_MEMORY_DATASETS
            .read()
            .unwrap()
            .get(&c.memory_ids[0])
            .expect("no such dataset")
            .get_vec()?;
        Ok(data)
    }
}

pub fn inspect() {
    println!("Collections:");
    for (id, c) in globals::COLLECTION_REGISTRY.read().unwrap().iter() {
        println!("  {}: {c:#?}", id);
    }

    println!("Functions:");
    for (id, f) in globals::PFN_REGISTRY.read().unwrap().iter() {
        println!("  {}: {}", id, f.name());
    }

    println!("Datasets:");
    for (id, d) in globals::IN_MEMORY_DATASETS.read().unwrap().iter() {
        println!("  {}: {} items", id, d.len());
    }
}

pub fn run() {
    // ??
    println!("TODO: run pipeline");
}
