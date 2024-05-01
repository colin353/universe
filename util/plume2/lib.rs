#![feature(min_specialization)]

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, RwLock};

mod in_memory;
mod iter;
mod map_fn;
mod sink;

use in_memory::InMemoryCollection;
use map_fn::{MapFn, MapFnWrapper};

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

pub fn get_collection(id: u64) -> CollectionSpec {
    globals::COLLECTION_REGISTRY
        .read()
        .expect("failed to acquire read lock")
        .get(&id)
        .expect("collection missing")
        .to_owned()
}

pub trait PFn: Send + Sync + 'static {
    fn plan(&self) -> (Plan, Vec<Plan>);
    fn execute(&self, plan: &Plan);
    fn name(&self) -> &'static str;
    fn init(&self, plan: &Plan) {}
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
    dependency: Option<Arc<dyn PFn>>,
    spec: CollectionSpec,

    _marker: std::marker::PhantomData<T>,
}

impl<T> Collection<T>
where
    T: Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            id: reserve_id(),
            dependency: None,
            spec: CollectionSpec::new(),
            _marker: std::marker::PhantomData {},
        }
    }

    pub fn plan(&self) -> Vec<Plan> {
        if let Some(f) = &self.dependency {
            let (mut s, mut deps) = f.plan();
            s.outputs.push(self.spec.clone());
            deps.push(s);
            return deps;
        }

        Vec::new()
    }

    pub fn from_vec(data: Vec<T>) -> Self {
        Self::from_vecs(vec![data])
    }

    pub fn from_vecs(datas: Vec<Vec<T>>) -> Self {
        let mut spec = CollectionSpec::new();
        spec.format = DataFormat::InMemory;
        spec.resolved = true;

        for data in datas {
            let memory_id = reserve_id();
            spec.memory_ids.push(memory_id);

            globals::IN_MEMORY_DATASETS
                .write()
                .unwrap()
                .insert(memory_id, InMemoryCollection::from_vec(data));
        }

        Self {
            id: reserve_id(),
            dependency: None,
            spec,
            _marker: std::marker::PhantomData {},
        }
    }

    pub fn from_recordio(path: &str) -> Self {
        let mut spec = CollectionSpec::new();
        spec.filenames.push(path.to_string());
        spec.resolved = true;
        spec.format = DataFormat::RecordIO;

        Self {
            id: reserve_id(),
            dependency: None,
            spec,
            _marker: std::marker::PhantomData {},
        }
    }

    pub fn map<O, M>(&self, f: M) -> Collection<O>
    where
        M: MapFn<Input = T, Output = O> + 'static,
        O: Send + Sync + 'static,
    {
        let mut out = Collection::new();
        out.dependency = Some(Arc::new(MapFnWrapper::new(self.id, Box::new(f))));
        out
    }
}
