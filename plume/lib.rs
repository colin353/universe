#![feature(specialization)]
#![feature(binary_heap_into_iter_sorted)]
#![feature(trait_alias)]

extern crate itertools;
extern crate plume_proto_rust;
extern crate primitive;
extern crate recordio;
extern crate shard_lib;
extern crate sstable;

use sstable::{reshard, SSTableBuilder, SSTableReader, ShardedSSTableReader};

#[macro_use]
extern crate lazy_static;

pub use itertools::StreamingIterator;
pub use primitive::{Primitive, PrimitiveType};

use itertools::MinHeap;
pub use itertools::KV;
pub use plume_proto_rust::*;
use primitive::Serializable;

use std::cmp::{Ordering, Reverse};
use std::collections::{BinaryHeap, HashMap};
use std::iter::Peekable;
use std::ops::Bound::*;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex, RwLock};

static ORDER: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;
static LAST_ID: AtomicU64 = AtomicU64::new(1);
static MAX_SSTABLE_HEAP_SIZE: usize = 100 * 1000 * 1000;

static TARGET_SHARDS: usize = 8;
static IN_MEMORY_RECORD_THRESHOLD: usize = 100 * 1000;
static IN_MEMORY_BYTES_THRESHOLD: u64 = 100 * 1000 * 1000;

lazy_static! {
    static ref PCOLLECTION_REGISTRY: RwLock<HashMap<u64, PCollectionProto>> =
        { RwLock::new(HashMap::new()) };
    static ref PFN_REGISTRY: RwLock<HashMap<u64, Arc<dyn PFn>>> = { RwLock::new(HashMap::new()) };
    static ref IN_MEMORY_DATASETS: RwLock<HashMap<u64, InMemoryPCollection>> =
        { RwLock::new(HashMap::new()) };
    static ref STAGES: RwLock<Vec<Stage>> = { RwLock::new(Vec::new()) };
    pub static ref RUNLOCK: Arc<Mutex<bool>> = { Arc::new(Mutex::new(false)) };
}

pub trait PlumeTrait = Send + Sync + Serializable + 'static;

fn reserve_id() -> u64 {
    LAST_ID.fetch_add(1, ORDER)
}

pub trait InMemoryPCollectionWrapper: Send + Sync {
    fn len(&self) -> usize;
    fn keyranges(&self, target_shards: usize) -> Vec<String>;
}

impl InMemoryPCollectionWrapper {
    pub fn downcast_ref<T: InMemoryPCollectionWrapper>(&self) -> Result<&T, ()> {
        // TODO: I don't know how to actually check if the types match. Need to do something like
        // verify the type ID. Basically if they don't match the program will do some undefined
        // behaviour so we should actually check and panic.
        unsafe { Ok(self.downcast_ref_unchecked()) }
    }

    pub unsafe fn downcast_ref_unchecked<T: InMemoryPCollectionWrapper>(&self) -> &T {
        &*(self as *const Self as *const T)
    }
}

pub struct InMemoryPCollectionUnderlying<T> {
    data: Arc<Vec<T>>,
}

pub struct InMemoryPTableUnderlying<T> {
    data: Arc<Vec<KV<String, T>>>,
}

impl<T> InMemoryPCollectionWrapper for InMemoryPTableUnderlying<T>
where
    T: PlumeTrait + Default,
{
    fn len(&self) -> usize {
        self.data.len()
    }

    fn keyranges(&self, target_shards: usize) -> Vec<String> {
        if self.data.len() == 0 {
            return Vec::new();
        }

        (1..(target_shards))
            .map(|i| {
                self.data[i * self.data.len() / target_shards]
                    .key()
                    .to_string()
            })
            .collect()
    }
}

impl<T> InMemoryPCollectionWrapper for InMemoryPCollectionUnderlying<T>
where
    T: Send + Sync + 'static,
{
    fn len(&self) -> usize {
        self.data.len()
    }

    fn keyranges(&self, target_shard: usize) -> Vec<String> {
        Vec::new()
    }
}

pub struct InMemoryPCollection {
    data: Arc<dyn InMemoryPCollectionWrapper>,
}

impl InMemoryPCollection {
    pub fn from_vec<T>(data: Vec<T>) -> Self
    where
        T: PlumeTrait + Default,
    {
        Self {
            data: Arc::new(InMemoryPCollectionUnderlying {
                data: Arc::new(data),
            }),
        }
    }

    pub fn from_table<T>(data: Vec<KV<String, T>>) -> Self
    where
        T: PlumeTrait + Default,
    {
        Self {
            data: Arc::new(InMemoryPTableUnderlying {
                data: Arc::new(data),
            }),
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct PCollection<T> {
    underlying: Arc<PCollectionUnderlying<T>>,
}

pub type PTable<T1, T2> = PCollection<KV<T1, T2>>;

pub struct PCollectionUnderlying<T> {
    id: AtomicU64,
    dependency: Option<Arc<dyn PFn>>,
    proto: PCollectionProto,
    _marker: std::marker::PhantomData<T>,
}

impl<T> PCollection<T>
where
    T: PlumeTrait + Default,
{
    fn new() -> Self {
        Self {
            underlying: Arc::new(PCollectionUnderlying::<T> {
                id: AtomicU64::new(0),
                dependency: None,
                proto: PCollectionProto::new(),
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    pub fn from_vec(data: Vec<T>) -> Self {
        Self::from_vecs(vec![data])
    }

    pub fn from_vecs(datas: Vec<Vec<T>>) -> Self {
        let mut config = PCollectionProto::new();
        config.set_format(DataFormat::IN_MEMORY);
        config.set_resolved(true);

        for data in datas {
            let memory_id = reserve_id();
            config.mut_memory_ids().push(memory_id);

            let data = InMemoryPCollection::from_vec(data);
            IN_MEMORY_DATASETS.write().unwrap().insert(memory_id, data);
        }

        Self {
            underlying: Arc::new(PCollectionUnderlying::<T> {
                id: AtomicU64::new(0),
                dependency: None,
                proto: config,
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    pub fn from_recordio(filename: &str) -> Self {
        let mut config = PCollectionProto::new();
        config.mut_filenames().push(filename.to_string());
        config.set_resolved(true);
        config.set_format(DataFormat::RECORDIO);

        Self {
            underlying: Arc::new(PCollectionUnderlying::<T> {
                id: AtomicU64::new(0),
                dependency: None,
                proto: config,
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    pub fn par_do<O, DoType>(&self, f: DoType) -> PCollection<O>
    where
        DoType: DoFn<Input = T, Output = O> + 'static,
        O: PlumeTrait + Default,
    {
        let mut out: PCollection<O> = PCollection::<O>::new();
        Arc::get_mut(&mut out.underlying).unwrap().dependency = Some(Arc::new(DoFnWrapper {
            dependency: self.clone(),
            function: Box::new(f),
        }));
        out
    }

    pub fn par_do_side_input<O, TSideInput, DoType>(
        &self,
        f: DoType,
        side_input: PCollection<TSideInput>,
    ) -> PCollection<O>
    where
        DoType: DoSideInputFn<Input = T, SideInput = TSideInput, Output = O> + 'static,
        TSideInput: PlumeTrait + Default,
        O: PlumeTrait + Default,
    {
        let mut out: PCollection<O> = PCollection::<O>::new();
        Arc::get_mut(&mut out.underlying).unwrap().dependency =
            Some(Arc::new(DoSideInputFnWrapper {
                dependency: self.clone(),
                side_input_dependency: side_input,
                function: Box::new(f),
            }));
        out
    }

    pub fn clone(&self) -> Self {
        PCollection {
            underlying: self.underlying.clone(),
        }
    }

    pub fn to_proto(&self) -> PCollectionProto {
        let mut out = self.underlying.proto.clone();
        out.set_id(self.underlying.id.load(ORDER));
        out
    }

    pub fn stages(&self) -> Vec<Stage> {
        let mut output = Vec::new();

        // If already registered, quit
        if self.underlying.id.load(ORDER) != 0 {
            return output;
        }

        let id = reserve_id();
        self.underlying.id.store(id, ORDER);

        PCOLLECTION_REGISTRY
            .write()
            .unwrap()
            .insert(id, self.to_proto());

        if let Some(ref f) = self.underlying.dependency {
            // Register the dependency
            let id = reserve_id();
            PFN_REGISTRY.write().unwrap().insert(id, f.clone());

            let (mut s, deps) = f.stages(id);
            output = deps;

            s.mut_outputs().push(self.to_proto());
            output.push(s);
        }

        output
    }

    pub fn into_vec(self) -> Arc<Vec<T>> {
        let reg = PCOLLECTION_REGISTRY.read().unwrap();
        let latest_pcoll = reg.get(&self.underlying.id.load(ORDER)).expect(&format!(
            "Couldn't find pcollection {}??",
            self.underlying.proto.get_id()
        ));
        if !latest_pcoll.get_resolved() {
            panic!("Can't run .into_vec() on a dataset that has not yet been computed",);
        }
        if latest_pcoll.get_memory_ids().len() != 1 {
            panic!(
                "I don't know how to run .into_vec() on a dataset with {} shards",
                latest_pcoll.get_memory_ids().len()
            );
        }
        let memory_id = latest_pcoll.get_memory_ids()[0];
        let guard = IN_MEMORY_DATASETS.read().unwrap();
        let dataset = guard
            .get(&memory_id)
            .expect(&format!("Failed to look up data in id={}", &memory_id));
        let pcoll: Arc<dyn InMemoryPCollectionWrapper> = dataset.data.clone();
        let data: &InMemoryPCollectionUnderlying<T> =
            pcoll.downcast_ref().expect("failed to downcast!");

        return data.data.clone();
    }

    pub fn write_to_vec(&mut self) {
        if self.underlying.proto.get_filenames().len() != 0
            || self.underlying.proto.get_format() != DataFormat::UNKNOWN
        {
            panic!("This ptable is already being written to disk!");
        }
        {
            let mut_underlying = Arc::get_mut(&mut self.underlying).unwrap();
            mut_underlying.proto.set_format(DataFormat::IN_MEMORY);
            mut_underlying.proto.set_target_memory_shards(1);
        }

        STAGES.write().unwrap().append(&mut self.stages());
    }
}

impl<T> PCollection<Primitive<T>>
where
    T: PrimitiveType,
    Primitive<T>: PlumeTrait + Default,
{
    pub fn from_primitive_vec(data: Vec<T>) -> Self {
        let converted: Vec<Primitive<T>> = data.into_iter().map(|x| x.into()).collect();
        Self::from_vec(converted)
    }

    pub fn from_primitive_vecs(data: Vec<Vec<T>>) -> Self {
        let converted: Vec<Vec<Primitive<T>>> = data
            .into_iter()
            .map(|x| x.into_iter().map(|y| y.into()).collect())
            .collect();
        Self::from_vecs(converted)
    }
}

impl<V> PCollection<KV<String, V>>
where
    V: PlumeTrait + Default,
{
    pub fn join<V2, O, JoinType>(self, right: PTable<String, V2>, f: JoinType) -> PCollection<O>
    where
        JoinType: JoinFn<ValueLeft = V, ValueRight = V2, Output = O> + 'static,
        V2: PlumeTrait + Default,
        O: PlumeTrait + Default,
    {
        let mut out = PCollection::<O>::new();
        Arc::get_mut(&mut out.underlying).unwrap().dependency = Some(Arc::new(JoinFnWrapper {
            dependency_left: Arc::new(self),
            dependency_right: Arc::new(right),
            function: Box::new(f),
        }));
        out
    }
}

impl<V> PCollection<KV<String, V>>
where
    V: PlumeTrait + Default,
    KV<String, V>: PlumeTrait + Default,
{
    pub fn from_sstable(filename: &str) -> Self {
        let mut config = PCollectionProto::new();
        config.mut_filenames().push(filename.to_string());
        config.set_resolved(true);
        config.set_format(DataFormat::SSTABLE);
        config.set_is_ptable(true);

        PCollection {
            underlying: Arc::new(PCollectionUnderlying::<KV<String, V>> {
                id: AtomicU64::new(0),
                dependency: None,
                proto: config,
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    pub fn from_table(data: Vec<KV<String, V>>) -> Self {
        Self::from_tables(vec![data])
    }

    pub fn from_tables(datas: Vec<Vec<KV<String, V>>>) -> Self {
        let mut config = PCollectionProto::new();
        config.set_format(DataFormat::IN_MEMORY);
        config.set_resolved(true);
        config.set_is_ptable(true);

        for data in datas {
            let memory_id = reserve_id();
            config.mut_memory_ids().push(memory_id);

            let data = InMemoryPCollection::from_table(data);
            IN_MEMORY_DATASETS.write().unwrap().insert(memory_id, data);
        }

        Self {
            underlying: Arc::new(PCollectionUnderlying::<KV<String, V>> {
                id: AtomicU64::new(0),
                dependency: None,
                proto: config,
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    pub fn write_to_sstable(&mut self, filename: &str) {
        if !self.underlying.proto.get_filenames().len() == 0
            || self.underlying.proto.get_format() != DataFormat::UNKNOWN
        {
            panic!("This ptable is already being written to disk!");
        }
        {
            let mut_underlying = Arc::get_mut(&mut self.underlying).unwrap();
            mut_underlying
                .proto
                .mut_filenames()
                .push(filename.to_string());
            mut_underlying.proto.set_format(DataFormat::SSTABLE);
        }

        STAGES.write().unwrap().append(&mut self.stages());
    }
}

impl<'a, V> PCollection<KV<String, V>>
where
    V: PlumeTrait + Default,
    KV<String, Stream<'a, V>>: Send + Sync + 'static,
{
    pub fn group_by_key_and_par_do<O, DoType>(&self, f: DoType) -> PCollection<O>
    where
        DoType: DoStreamFn<Input = V, Output = O> + 'static,
        O: PlumeTrait + Default,
    {
        let mut out: PCollection<O> = PCollection::<O>::new();
        Arc::get_mut(&mut out.underlying).unwrap().dependency = Some(Arc::new(DoStreamFnWrapper {
            dependency: self.clone(),
            function: Box::new(f),
        }));
        out
    }
}

pub fn update_stage(stage: &mut Stage) {
    for input in stage.mut_inputs().iter_mut() {
        let reg = PCOLLECTION_REGISTRY.read().unwrap();
        let latest_input = reg.get(&input.get_id()).unwrap();

        *input = latest_input.clone();
    }

    for input in stage.mut_side_inputs().iter_mut() {
        let reg = PCOLLECTION_REGISTRY.read().unwrap();
        let latest_input = reg.get(&input.get_id()).unwrap();

        *input = latest_input.clone();
    }

    for output in stage.mut_outputs().iter_mut() {
        let reg = PCOLLECTION_REGISTRY.read().unwrap();
        let latest_input = reg.get(&output.get_id()).unwrap();

        *output = latest_input.clone();
    }
}

pub fn run() {
    let mut stages = STAGES.read().unwrap().clone();
    let mut started = std::collections::HashSet::new();
    let planner = Planner::new();
    let pool = pool::ThreadPool::new(4);
    let mut completed = 0;
    let mut total_shards = stages.len();
    let mut started_shards = 0;

    loop {
        let mut did_execute = false;
        for (id, stage) in stages.iter_mut().enumerate() {
            update_stage(stage);

            if started.contains(&id) {
                continue;
            }

            let mut ready = true;
            for input in stage.get_inputs() {
                if !input.get_resolved() {
                    ready = false;
                }
            }

            for input in stage.get_side_inputs() {
                if !input.get_resolved() {
                    ready = false;
                }
            }

            if ready {
                let shards = planner.plan(&stage);
                total_shards += shards.len() - 1;

                // Update the total number of shards in the pcollection regsitry. Note,
                // this means we only support a single sharded output per stage.
                if shards.len() > 0 && shards[0].get_outputs().len() > 0 {
                    let mut pcoll_write = PCOLLECTION_REGISTRY.write().unwrap();
                    let mut config = pcoll_write
                        .get_mut(&shards[0].get_outputs()[0].get_id())
                        .unwrap();
                    config.set_num_shards(shards.len() as u64);
                    config.set_num_resolved_shards(0);
                }

                if shards.len() == 0 {
                    panic!("Stage {} has zero shards!", shards.len());
                }

                started_shards += shards.len();
                for shard in shards {
                    let s = shard.clone();
                    pool.execute(move || {
                        execute_shard(&s);
                    })
                }
                did_execute = true;
                started.insert(id);
            }
        }

        println!("{}/{} stages completed", completed, total_shards);

        if !did_execute {
            if pool.get_in_progress() == 0 && started_shards == total_shards {
                pool.join();
                println!("we're done!");
                break;
            }

            if completed > total_shards {
                println!("something went wrong");
                break;
            }
        }

        pool.block_until_job_completes();
        completed += 1;
    }
}

pub fn cleanup() {
    let mut stages = STAGES.write().unwrap();
    stages.clear();
    let mut pfns = PFN_REGISTRY.write().unwrap();
    pfns.clear();
    let mut pcolls = PCOLLECTION_REGISTRY.write().unwrap();
    pcolls.clear();
    let mut data = IN_MEMORY_DATASETS.write().unwrap();
    data.clear();
    LAST_ID.store(1, ORDER);
}

pub fn execute_shard(shard: &Shard) {
    let pfn = PFN_REGISTRY
        .read()
        .unwrap()
        .get(&shard.get_function().get_id())
        .unwrap()
        .clone();

    println!("executing: {}", pfn.name());

    pfn.init(shard);
    pfn.execute(shard);
}

pub trait PFn: Send + Sync {
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>);
    fn execute(&self, shard: &Shard);
    fn name(&self) -> &'static str;
    fn init(&self, shard: &Shard) {}
}
pub trait EmitFn<T> {
    fn emit(&mut self, value: T);
    fn finish(self: Box<Self>);
}

pub struct JoinFnWrapper<V1, V2, O> {
    dependency_left: Arc<PTable<String, V1>>,
    dependency_right: Arc<PTable<String, V2>>,
    function: Box<JoinFn<ValueLeft = V1, ValueRight = V2, Output = O>>,
}

pub struct Stream<'a, T> {
    iter: &'a mut (dyn StreamingIterator<Item = KV<String, T>>),
    current_key: Option<String>,
}

unsafe impl<'a, T> Send for Stream<'a, T> where T: Send {}
unsafe impl<'a, T> Sync for Stream<'a, T> where T: Sync {}

impl<'a, T> Stream<'a, T> {
    fn new(iter: &'a mut dyn StreamingIterator<Item = KV<String, T>>) -> Self {
        let current_key = iter.peek().map(|x| x.0.to_string());

        Self {
            iter: iter,
            current_key: current_key,
        }
    }
}

impl<'a, T> StreamingIterator for Stream<'a, T> {
    type Item = T;

    fn peek(&mut self) -> Option<&Self::Item> {
        let key = match &self.current_key {
            Some(x) => x,
            None => return None,
        };

        if let Some(x) = self.iter.peek() {
            if x.key() == key {
                return self.iter.next().map(|x| x.value());
            }
        }

        None
    }

    fn next(&mut self) -> Option<&Self::Item> {
        let key = match &self.current_key {
            Some(x) => x,
            None => return None,
        };

        if let Some(x) = self.iter.peek() {
            if &x.0 == key {
                return self.iter.next().map(|x| &x.1);
            }
        }

        None
    }
}

pub trait JoinFn: Send + Sync {
    type ValueLeft;
    type ValueRight;
    type Output;
    fn join(
        &self,
        key: &str,
        left: &mut Stream<Self::ValueLeft>,
        right: &mut Stream<Self::ValueRight>,
        emit: &mut dyn EmitFn<Self::Output>,
    );

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }
}

pub struct DoSideInputFnWrapper<TInput, TSideInput, TOutput> {
    dependency: PCollection<TInput>,
    side_input_dependency: PCollection<TSideInput>,
    function: Box<DoSideInputFn<Input = TInput, SideInput = TSideInput, Output = TOutput>>,
}

pub struct DoFnWrapper<T1, T2> {
    dependency: PCollection<T1>,
    function: Box<DoFn<Input = T1, Output = T2>>,
}
pub trait DoFn: Send + Sync {
    type Input;
    type Output;
    fn do_it(&self, input: &Self::Input, emit: &mut dyn EmitFn<Self::Output>);

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }
}

pub trait DoSideInputFn: Send + Sync {
    type Input;
    type SideInput;
    type Output;
    fn init(&self, side_input: &mut dyn StreamingIterator<Item = Self::SideInput>);
    fn do_it(&self, input: &Self::Input, emit: &mut dyn EmitFn<Self::Output>);

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }
}

pub struct DoStreamFnWrapper<T1, T2> {
    dependency: PCollection<KV<String, T1>>,
    function: Box<DoStreamFn<Input = T1, Output = T2>>,
}
pub trait DoStreamFn: Send + Sync {
    type Input;
    type Output;
    fn do_it(
        &self,
        key: &str,
        values: &mut Stream<Self::Input>,
        emit: &mut dyn EmitFn<Self::Output>,
    );

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }
}

impl<T1, T2> PFn for DoStreamFnWrapper<T1, T2>
where
    T1: PlumeTrait + Default,
    T2: PlumeTrait + Default,
{
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>) {
        let dep_stages = self.dependency.stages();

        let mut s = Stage::new();
        s.mut_inputs().push(self.dependency.to_proto());
        s.mut_function().set_description(String::from("DoStreamFn"));
        s.mut_function().set_id(id);

        (s, dep_stages)
    }

    fn name(&self) -> &'static str {
        self.function.type_name()
    }

    default fn execute(&self, shard: &Shard) {
        for input in shard.get_inputs() {
            let producer = SinkProducer::<T2>::new();
            let mut sink = producer.make_sink(shard.get_outputs());
            {
                let sink_ref: &mut dyn EmitFn<T2> = &mut *sink;

                match input.get_format() {
                    DataFormat::IN_MEMORY => {
                        let source = Source::<KV<String, Stream<'_, T1>>>::new(input.clone());
                        let s = source.mem_table_grouped_source();
                        let mut iter = s.iter();
                        let mut dyn_iterator: &mut dyn StreamingIterator<Item = KV<String, T1>> =
                            &mut iter;
                        loop {
                            let key = if let Some(kv) = dyn_iterator.peek() {
                                kv.0.clone()
                            } else {
                                break;
                            };
                            {
                                let mut s = Stream::new(dyn_iterator);
                                self.function.do_it(&key, &mut s, sink_ref);
                                s.count();
                            }
                        }
                    }
                    DataFormat::SSTABLE => {
                        let source = Source::<KV<String, T1>>::new(input.clone());
                        let mut iter = source.sstable_source();
                        let mut dyn_iterator: &mut dyn StreamingIterator<Item = KV<String, T1>> =
                            &mut iter;
                        loop {
                            let key = if let Some(kv) = dyn_iterator.peek() {
                                kv.0.clone()
                            } else {
                                break;
                            };
                            {
                                let mut s = Stream::new(dyn_iterator);
                                self.function.do_it(&key, &mut s, sink_ref);
                                s.count();
                            }
                        }
                    }
                    x => {
                        panic!("I don't know how to read input type {:?}!", x);
                    }
                }
            }
            sink.finish();
        }
    }
}

impl<TInput, TSideInput, TOutput> PFn for DoSideInputFnWrapper<TInput, TSideInput, TOutput>
where
    TInput: PlumeTrait + Default,
    TSideInput: PlumeTrait + Default,
    TOutput: PlumeTrait + Default,
{
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>) {
        let mut dep_stages = self.dependency.stages();
        dep_stages.append(&mut self.side_input_dependency.stages());

        let mut s = Stage::new();
        s.mut_inputs().push(self.dependency.to_proto());
        s.mut_function().set_description(String::from("DoFn"));
        s.mut_function().set_id(id);
        s.mut_side_inputs()
            .push(self.side_input_dependency.to_proto());

        (s, dep_stages)
    }

    fn name(&self) -> &'static str {
        self.function.type_name()
    }

    fn init(&self, input: &Shard) {
        let side_input = &input.get_side_inputs()[0];

        let side_input = {
            let id = &input.get_side_inputs()[0].get_id();
            let reg = PCOLLECTION_REGISTRY.read().unwrap();
            reg.get(id)
                .expect(&format!("Couldn't find pcollection {}??", id))
                .clone()
        };

        let source = Source::<TSideInput>::new(side_input.clone());
        let mut mem_iter;
        let mut mem_src;
        let mut rec_src;
        let mut rec_iter;
        let mut sst_src;
        let mut dyn_src: &mut dyn StreamingIterator<Item = TSideInput>;
        match side_input.get_format() {
            DataFormat::IN_MEMORY => {
                mem_src = source.mem_source();
                mem_iter = mem_src.iter();
                dyn_src = &mut mem_iter;
            }
            DataFormat::RECORDIO => {
                rec_src = source.recordio_source();
                rec_iter = rec_src.streaming_iter();
                dyn_src = &mut rec_iter;
            }
            DataFormat::SSTABLE => {
                sst_src = source.sstable_source_or_panic();
                dyn_src = Box::leak(sst_src);
            }
            _ => {
                panic!("idk how to deal with side input {:?}", side_input);
            }
        }
        self.function.init(dyn_src);
    }

    default fn execute(&self, shard: &Shard) {
        for input in shard.get_inputs() {
            let source = Source::<TInput>::new(input.clone());
            let producer = SinkProducer::<TOutput>::new();
            let mut sink = producer.make_sink(shard.get_outputs());
            {
                let sink_ref: &mut dyn EmitFn<TOutput> = &mut *sink;

                match input.get_format() {
                    DataFormat::IN_MEMORY => {
                        let mut memsource = source.mem_source();
                        let mut source_iter = memsource.iter();
                        while let Some(item) = source_iter.next() {
                            self.function.do_it(item, sink_ref);
                        }
                    }
                    DataFormat::RECORDIO => {
                        let mut recordio_source = source.recordio_source();
                        while let Some(item) = recordio_source.next() {
                            self.function.do_it(&item, sink_ref);
                        }
                    }
                    x => panic!("I don't know how to execute format: {:?}!", x),
                }
            }
            sink.finish();
        }
    }
}

impl<TInput, TSideInput, TOutput> PFn
    for DoSideInputFnWrapper<KV<String, TInput>, TSideInput, TOutput>
where
    TInput: PlumeTrait + Default,
    TSideInput: PlumeTrait + Default,
    TOutput: PlumeTrait + Default,
{
    default fn execute(&self, shard: &Shard) {
        for input in shard.get_inputs() {
            let source = Source::<KV<String, TInput>>::new(input.clone());
            let producer = SinkProducer::<TOutput>::new();
            let mut sink = producer.make_sink(shard.get_outputs());
            {
                let sink_ref: &mut dyn EmitFn<TOutput> = &mut *sink;
                match input.get_format() {
                    DataFormat::IN_MEMORY => {
                        let memtable = source.mem_table_source();
                        let mut iter = memtable.iter();
                        while let Some(item) = iter.next() {
                            self.function.do_it(item, sink_ref);
                        }
                    }
                    DataFormat::SSTABLE => {
                        let mut iter = source.sstable_source();
                        while let Some((key, value)) = iter.next() {
                            let kv = KV(key, value);
                            self.function.do_it(&kv, sink_ref);
                        }
                    }
                    x => {
                        panic!("I don't know how to execute with source type: {:?}", x);
                    }
                }
            }
            sink.finish();
        }
    }
}

impl<T1, T2> PFn for DoFnWrapper<T1, T2>
where
    T1: PlumeTrait + Default,
    T2: PlumeTrait + Default,
{
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>) {
        let dep_stages = self.dependency.stages();

        let mut s = Stage::new();
        s.mut_inputs().push(self.dependency.to_proto());
        s.mut_function().set_description(String::from("DoFn"));
        s.mut_function().set_id(id);

        (s, dep_stages)
    }

    fn name(&self) -> &'static str {
        self.function.type_name()
    }

    default fn execute(&self, shard: &Shard) {
        for input in shard.get_inputs() {
            let source = Source::<T1>::new(input.clone());
            let producer = SinkProducer::<T2>::new();
            let mut sink = producer.make_sink(shard.get_outputs());
            {
                let sink_ref: &mut dyn EmitFn<T2> = &mut *sink;

                match input.get_format() {
                    DataFormat::IN_MEMORY => {
                        let mut memsource = source.mem_source();
                        let mut source_iter = memsource.iter();
                        while let Some(item) = source_iter.next() {
                            self.function.do_it(item, sink_ref);
                        }
                    }
                    DataFormat::RECORDIO => {
                        let mut recordio_source = source.recordio_source();
                        while let Some(item) = recordio_source.next() {
                            self.function.do_it(&item, sink_ref);
                        }
                    }
                    x => panic!("I don't know how to execute format: {:?}!", x),
                }
            }
            sink.finish();
        }
    }
}

impl<T1, T2> PFn for DoFnWrapper<KV<String, T1>, T2>
where
    T1: PlumeTrait + Default,
    T2: PlumeTrait + Default,
{
    default fn execute(&self, shard: &Shard) {
        for input in shard.get_inputs() {
            let source = Source::<KV<String, T1>>::new(input.clone());
            let producer = SinkProducer::<T2>::new();
            let mut sink = producer.make_sink(shard.get_outputs());
            {
                let sink_ref: &mut dyn EmitFn<T2> = &mut *sink;

                match input.get_format() {
                    DataFormat::IN_MEMORY => {
                        let memtable = source.mem_table_source();
                        let mut iter = memtable.iter();
                        while let Some(item) = iter.next() {
                            self.function.do_it(item, sink_ref);
                        }
                    }
                    DataFormat::SSTABLE => {
                        let mut iter = source.sstable_source();
                        while let Some((key, value)) = iter.next() {
                            let kv = KV(key, value);
                            self.function.do_it(&kv, sink_ref);
                        }
                    }
                    x => {
                        panic!("I don't know how to execute with source type: {:?}", x);
                    }
                }
            }
            sink.finish();
        }
    }
}

enum JoinTask {
    Left,
    Right,
    Both,
}

impl<V1, V2, O> PFn for JoinFnWrapper<V1, V2, O>
where
    V1: PlumeTrait + Default,
    V2: PlumeTrait + Default,
    O: PlumeTrait + Default,
{
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>) {
        let mut s = Stage::new();
        let mut deps = self.dependency_left.stages();
        deps.append(&mut self.dependency_right.stages());

        s.mut_inputs().push(self.dependency_left.to_proto());
        s.mut_inputs().push(self.dependency_right.to_proto());
        s.mut_function().set_description(String::from("JoinFn"));
        s.mut_function().set_id(id);

        (s, deps)
    }

    fn name(&self) -> &'static str {
        self.function.type_name()
    }

    fn execute(&self, shard: &Shard) {
        if shard.get_inputs().len() != 2 {
            panic!(
                "[{}] I don't know how to join {} inputs!",
                self.name(),
                shard.get_inputs().len()
            );
        }
        let left = &shard.get_inputs()[0];
        let right = &shard.get_inputs()[1];

        let source_left = Source::<KV<String, Stream<'_, V1>>>::new(left.clone());
        let source_right = Source::<KV<String, Stream<'_, V2>>>::new(right.clone());
        let producer = SinkProducer::<O>::new();
        let mut sink = producer.make_sink(shard.get_outputs());
        {
            let sink_ref: &mut dyn EmitFn<O> = &mut *sink;

            let s_left_mem;
            let mut s_left_mem_iter;
            let mut s_left_sst;
            let mut dyn_left_iter: &mut dyn StreamingIterator<Item = KV<String, V1>>;
            if left.get_format() == DataFormat::IN_MEMORY {
                s_left_mem = source_left.mem_table_grouped_source();
                s_left_mem_iter = s_left_mem.iter();
                dyn_left_iter = &mut s_left_mem_iter;
            } else if left.get_format() == DataFormat::SSTABLE {
                s_left_sst = source_left.sstable_source();
                dyn_left_iter = &mut s_left_sst;
            } else {
                panic!(
                    "[{}] I don't know how to join with left input data format {:?}!\n{:?}",
                    self.name(),
                    left.get_format(),
                    shard,
                );
            }

            let mut empty_left = InMemoryTableSourceIteratorWrapper::<V1>::empty();
            let mut empty_left_iter = empty_left.iter();
            let mut dyn_left_empty: &mut dyn StreamingIterator<Item = KV<String, V1>> =
                &mut empty_left_iter;

            let s_right_mem;
            let mut s_right_mem_iter;
            let mut s_right_sst;
            let mut dyn_right_iter: &mut dyn StreamingIterator<Item = KV<String, V2>>;
            if right.get_format() == DataFormat::IN_MEMORY {
                s_right_mem = source_right.mem_table_grouped_source();
                s_right_mem_iter = s_right_mem.iter();
                dyn_right_iter = &mut s_right_mem_iter;
            } else if right.get_format() == DataFormat::SSTABLE {
                s_right_sst = source_right.sstable_source();
                dyn_right_iter = &mut s_right_sst;
            } else {
                panic!(
                    "[{}] I don't know how to join with right input data format {:?}!\n{:?}",
                    self.name(),
                    right.get_format(),
                    shard
                );
            }

            let mut empty_right = InMemoryTableSourceIteratorWrapper::<V2>::empty();
            let mut empty_right_iter = empty_right.iter();
            let mut dyn_right_empty: &mut dyn StreamingIterator<Item = KV<String, V2>> =
                &mut empty_right_iter;

            loop {
                let mut key;
                let mut task = match (dyn_left_iter.peek(), dyn_right_iter.peek()) {
                    (Some(l), Some(r)) => {
                        if l.key() == r.key() {
                            key = l.key().to_string();
                            JoinTask::Both
                        } else if l.key() < r.key() {
                            key = l.key().to_string();
                            JoinTask::Left
                        } else {
                            key = r.key().to_string();
                            JoinTask::Right
                        }
                    }
                    (Some(l), None) => {
                        key = l.key().to_string();
                        JoinTask::Both
                    }
                    (None, Some(r)) => {
                        key = r.key().to_string();
                        JoinTask::Both
                    }
                    (None, None) => break,
                };

                match task {
                    JoinTask::Both => {
                        let mut s_l = Stream::new(dyn_left_iter);
                        let mut s_r = Stream::new(dyn_right_iter);
                        self.function.join(&key, &mut s_l, &mut s_r, sink_ref);
                        s_l.count();
                        s_r.count();
                    }
                    JoinTask::Right => {
                        let mut s_l = Stream::new(dyn_left_empty);
                        let mut s_r = Stream::new(dyn_right_iter);
                        self.function.join(&key, &mut s_l, &mut s_r, sink_ref);
                        s_r.count();
                    }
                    JoinTask::Left => {
                        let mut s_l = Stream::new(dyn_left_iter);
                        let mut s_r = Stream::new(dyn_right_empty);
                        self.function.join(&key, &mut s_l, &mut s_r, sink_ref);
                        s_l.count();
                    }
                }
            }

            sink.finish();
        }
    }
}

struct Planner {
    target_shards: usize,
    in_memory_record_threshold: usize,
    in_memory_bytes_threshold: u64,

    temp_data_folder: String,
}

impl Planner {
    pub fn new() -> Self {
        Self {
            target_shards: TARGET_SHARDS,
            in_memory_record_threshold: IN_MEMORY_RECORD_THRESHOLD,
            in_memory_bytes_threshold: IN_MEMORY_BYTES_THRESHOLD,
            temp_data_folder: String::from("/tmp/"),
        }
    }

    pub fn plan(&self, stage: &Stage) -> Vec<Shard> {
        if stage.get_outputs().len() != 1 {
            panic!(
                "I don't know how to plan for {} outputs",
                stage.get_outputs().len()
            );
        }

        let mut output = stage.get_outputs()[0].clone();

        // First, look at the output. If it has a specific shard count, we should stick to that.
        let mut target_shards = self.target_shards;
        if let Some(shard_count) = Self::count_shards(&output) {
            target_shards = shard_count;
        }

        // Now let's try to construct the input specifications
        let mut shards = Vec::new();

        if stage.get_inputs().len() > 2 {
            panic!(
                "I don't know how to plan for {} inputs",
                stage.get_inputs().len()
            );
        }

        if output.get_format() == DataFormat::UNKNOWN {
            // If the output is not defined, we should decide what to use. If the
            // data is not too big, we will keep it in memory, else write to disk
            let size = Self::estimate_size(stage.get_inputs());
            if self.keep_in_memory(&size) {
                println!(
                    "Kept {:?} in memory because of small size estimate {:?}",
                    output, size
                );
                output.set_format(DataFormat::IN_MEMORY);
            } else {
                output.set_format(DataFormat::SSTABLE);
            }
        }

        let sharded_inputs = self.shard_inputs(stage.get_inputs(), target_shards);
        let sharded_outputs = self.shard_output(&output, sharded_inputs.len());

        if sharded_inputs.len() != sharded_outputs.len() {
            panic!(
                "Can't plan: got {} inputs and {} outputs!",
                sharded_inputs.len(),
                sharded_outputs.len(),
            );
        }

        for (shard_inputs, shard_output) in
            sharded_inputs.into_iter().zip(sharded_outputs.into_iter())
        {
            let mut shard = Shard::new();
            for shard_input in shard_inputs {
                shard.mut_inputs().push(shard_input);
            }
            shard.set_function(stage.get_function().clone());
            shard.mut_outputs().push(shard_output);

            for side_input in stage.get_side_inputs() {
                shard.mut_side_inputs().push(side_input.clone());
            }

            shards.push(shard);
        }

        shards
    }

    fn shard_output(
        &self,
        output: &PCollectionProto,
        target_shards: usize,
    ) -> Vec<PCollectionProto> {
        let mut shards = Vec::new();
        if output.get_format() == DataFormat::IN_MEMORY {
            for _ in 0..target_shards {
                let mut s = output.clone();
                shards.push(s);
            }
            return shards;
        }

        if output.get_format() == DataFormat::SSTABLE {
            let mut sharded_filenames = if let Some(s) = output.get_filenames().get(0) {
                shard_lib::unshard(s)
            } else {
                // We haven't determined a filename for this output. That means it will be resolved
                // now, and we have to update the pcollection registry with the concrete output
                // info.

                let mut pcoll_write = PCOLLECTION_REGISTRY.write().unwrap();
                let mut config = pcoll_write.get_mut(&output.get_id()).unwrap();
                let filenames = shard_lib::unshard(&format!(
                    "{}/output{:02}.sstable@{}",
                    self.temp_data_folder,
                    output.get_id(),
                    target_shards
                ));

                for filename in &filenames {
                    config.mut_filenames().push(filename.clone());
                }
                filenames
            };

            for (index, filename) in sharded_filenames.into_iter().enumerate() {
                let mut s = output.clone();
                s.mut_filenames().clear();
                s.mut_filenames().push(filename);
                s.set_temporary_path(format!(
                    "{}{}/{}",
                    self.temp_data_folder,
                    output.get_id(),
                    index
                ));
                shards.push(s);
            }

            return shards;
        }

        if output.get_format() == DataFormat::RECORDIO {
            let sharded_filename = output.get_filenames()[0].to_string();
            if sharded_filename.is_empty() {
                let sharded_filename = format!(
                    "{}/output{:02}.recordio@{}",
                    self.temp_data_folder,
                    output.get_id(),
                    target_shards
                );
            }
            for filename in shard_lib::unshard(&sharded_filename) {
                let mut s = output.clone();
                s.mut_filenames().clear();
                s.mut_filenames().push(filename);
                shards.push(s);
            }

            return shards;
        }

        panic!(
            "I don't know how to shard output for type {:?}!",
            output.get_format()
        );
    }

    fn keep_in_memory(&self, size: &SizeEstimate) -> bool {
        if size.get_very_big() {
            return false;
        }

        if size.get_records() > self.in_memory_record_threshold as u64 {
            return false;
        }

        if size.get_data_bytes() > self.in_memory_bytes_threshold {
            return false;
        }

        true
    }

    pub fn estimate_size(inputs: &[PCollectionProto]) -> SizeEstimate {
        let mut out = SizeEstimate::new();
        let mut count = 0;
        for input in inputs {
            if input.get_format() == DataFormat::IN_MEMORY {
                for memory_id in input.get_memory_ids() {
                    let mem_reader = IN_MEMORY_DATASETS.read().unwrap();
                    count += mem_reader.get(memory_id).unwrap().len();
                }
            }
            if input.get_format() == DataFormat::SSTABLE
                || input.get_format() == DataFormat::RECORDIO
            {
                // TODO: look at the filesize and use that to estimate size
                out.set_very_big(true);
            }
        }
        out.set_records(count as u64);
        out
    }

    pub fn count_shards(input: &PCollectionProto) -> Option<usize> {
        if input.get_format() == DataFormat::IN_MEMORY {
            if input.get_memory_ids().len() > 0 {
                return Some(input.get_memory_ids().len());
            } else if input.get_target_memory_shards() > 0 {
                return Some(input.get_target_memory_shards() as usize);
            } else {
                return None;
            }
        }

        if input.get_format() == DataFormat::RECORDIO || input.get_format() == DataFormat::SSTABLE {
            if input.get_filenames()[0].is_empty() {
                return None;
            }
            return Some(shard_lib::unshard(&input.get_filenames()[0]).len());
        }

        None
    }

    fn shard_inputs(
        &self,
        inputs: &[PCollectionProto],
        target_shards: usize,
    ) -> Vec<Vec<PCollectionProto>> {
        if inputs.len() == 0 {
            panic!("Can't shard with zero inputs!");
        }

        // RecordIO doesn't support keyrange sharding, so we have to just use whatever sharding
        // strategy was present on the input.
        if inputs[0].get_format() == DataFormat::RECORDIO {
            if inputs.len() > 1 {
                panic!("Can't have multiple RECORDIO inputs!");
            }
            let input = &inputs[0];
            return shard_lib::unshard(&input.get_filenames()[0])
                .iter()
                .map(|f| {
                    let mut s = input.clone();
                    s.mut_filenames().clear();
                    s.mut_filenames().push(f.to_string());
                    vec![s]
                })
                .collect();
        }

        if inputs[0].get_format() == DataFormat::SSTABLE {
            let mut boundaries = Vec::new();
            for input in inputs {
                let mut filenames = Vec::new();
                for file in input.get_filenames() {
                    filenames.append(&mut shard_lib::unshard(file));
                }
                let reader = ShardedSSTableReader::<Primitive<Vec<u8>>>::from_filenames(
                    &filenames,
                    "",
                    String::new(),
                )
                .unwrap();
                boundaries.append(&mut reader.get_shard_boundaries(target_shards));
            }

            let mut boundaries = shard_lib::compact_shards(boundaries, target_shards);
            boundaries.push(String::new());
            boundaries.insert(0, String::new());

            let mut results = Vec::new();
            for window in boundaries.windows(2) {
                let mut shard = Vec::new();
                for input in inputs {
                    let mut out = input.clone();
                    out.mut_filenames().clear();
                    for sharded_file in input.get_filenames() {
                        for unsharded_file in shard_lib::unshard(sharded_file) {
                            out.mut_filenames().push(unsharded_file);
                        }
                    }

                    out.set_starting_key(window[0].to_string());
                    out.set_ending_key(window[1].to_string());

                    shard.push(out);
                }
                results.push(shard);
            }
            return results;
        }

        if inputs[0].get_format() == DataFormat::IN_MEMORY && inputs[0].get_is_ptable() {
            let mut boundaries = Self::shard_memtables(inputs, target_shards);
            boundaries.push(String::new());
            boundaries.insert(0, String::new());

            let mut results = Vec::new();
            for window in boundaries.windows(2) {
                let mut shard = Vec::new();
                for input in inputs {
                    let mut out = input.clone();
                    out.set_starting_key(window[0].to_string());
                    out.set_ending_key(window[1].to_string());
                    shard.push(out);
                }
                results.push(shard);
            }
            return results;
        }

        if inputs[0].get_format() == DataFormat::IN_MEMORY {
            if inputs.len() > 1 {
                panic!("I don't know how to shard multiple non-ptable inputs!");
            }
            let input = &inputs[0];
            let input_shards = input.get_memory_ids().len();

            // If we want to expand the number of shards, we should split the
            // existing shards into pieces
            if target_shards > input_shards {
                let mut output = Vec::new();
                for (index, memory_id) in input.get_memory_ids().iter().enumerate() {
                    let mut s = input.clone();
                    s.mut_memory_ids().clear();
                    s.mut_memory_ids().push(*memory_id);
                    let extra = if index < (target_shards % input_shards) {
                        1
                    } else {
                        0
                    };
                    output.append(&mut Self::split_memshard(
                        &s,
                        (target_shards / input_shards) + extra,
                    ));
                }
                return output.into_iter().map(|x| vec![x]).collect();
            }

            // We have more shards than we want, so we need to group them.
            let mut memory_id_iter = input.get_memory_ids().iter();
            let mut output = Vec::new();
            for index in 0..target_shards {
                let mut s = input.clone();
                s.mut_memory_ids().clear();

                let extra = if index < (input_shards % target_shards) {
                    1
                } else {
                    0
                };
                let num_to_take = input_shards / target_shards + extra;
                for memory_id in 0..num_to_take {
                    s.mut_memory_ids().push(*memory_id_iter.next().unwrap());
                }
                output.push(vec![s]);
            }

            return output;
        }

        Vec::new()
    }

    fn shard_memtables(inputs: &[PCollectionProto], target_shards: usize) -> Vec<String> {
        let mem_reader = IN_MEMORY_DATASETS.read().unwrap();
        let mut shard_boundaries = Vec::new();
        for input in inputs {
            for memory_id in input.get_memory_ids() {
                shard_boundaries.append(
                    &mut mem_reader
                        .get(&memory_id)
                        .unwrap()
                        .data
                        .keyranges(target_shards),
                );
            }
        }
        shard_lib::compact_shards(shard_boundaries, target_shards - 1)
    }

    fn split_memshard(input: &PCollectionProto, target_shards: usize) -> Vec<PCollectionProto> {
        let memory_id = input.get_memory_ids()[0];
        let mem_reader = IN_MEMORY_DATASETS.read().unwrap();
        let data_len = mem_reader.get(&memory_id).unwrap().len();
        let target_shards = std::cmp::min(target_shards, data_len);
        return (0..target_shards)
            .map(|i| {
                let mut s = input.clone();
                s.set_starting_index((i * data_len / target_shards) as u64);
                if i == target_shards - 1 {
                    s.set_ending_index(0);
                } else {
                    s.set_ending_index(((i + 1) * data_len / target_shards) as u64);
                }
                s
            })
            .collect();
    }
}

struct SinkProducer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> SinkProducer<T> {
    fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData {},
        }
    }
}

trait SinkProducerTrait<T> {
    fn make_sink(&self, outputs: &[PCollectionProto]) -> Box<dyn EmitFn<T>>;
}

impl<T> SinkProducerTrait<T> for SinkProducer<T>
where
    T: PlumeTrait + Default,
{
    default fn make_sink(&self, outputs: &[PCollectionProto]) -> Box<dyn EmitFn<T>> {
        if outputs.len() == 0 {
            panic!("Can't make a sink from zero outputs");
        }

        if outputs[0].get_format() == DataFormat::IN_MEMORY {
            return Box::new(MemorySink::new(outputs));
        }

        panic!(
            "I don't know how to make a sink for {:?}",
            outputs[0].get_format()
        );
    }
}

impl<T> SinkProducerTrait<KV<String, T>> for SinkProducer<KV<String, T>>
where
    T: PlumeTrait + Default,
{
    fn make_sink(&self, outputs: &[PCollectionProto]) -> Box<dyn EmitFn<KV<String, T>>> {
        if outputs.len() == 0 {
            panic!("Can't make a sink from zero outputs");
        }

        if outputs[0].get_format() == DataFormat::IN_MEMORY {
            return Box::new(OrderedMemorySink::<T>::new(outputs));
        }

        if outputs[0].get_format() == DataFormat::SSTABLE {
            return Box::new(SSTableSink::<T>::new(outputs));
        }

        panic!(
            "I don't know how to make a sink for {:?}",
            outputs[0].get_format()
        );
    }
}

struct SSTableSink<T> {
    configs: Vec<PCollectionProto>,
    heap: MinHeap<KV<String, T>>,
    sstables_written: Vec<String>,
}

impl<T> SSTableSink<T>
where
    T: PlumeTrait + Default,
{
    pub fn new(configs: &[PCollectionProto]) -> Self {
        if configs.len() == 0 {
            panic!("Can't construct SSTable sink with zero outputs!");
        }
        Self {
            configs: configs.to_vec(),
            sstables_written: Vec::new(),
            heap: MinHeap::new(),
        }
    }

    pub fn get_filename(idx: usize, path: &str) -> String {
        format!("{}/{}.sstable", path, idx)
    }

    pub fn flush(&mut self) {
        std::fs::create_dir_all(self.configs[0].get_temporary_path()).unwrap();
        let path = Self::get_filename(
            self.sstables_written.len(),
            self.configs[0].get_temporary_path(),
        );
        self.sstables_written.push(path.clone());
        let file = std::fs::File::create(path).unwrap();
        let mut writer = std::io::BufWriter::new(file);
        let mut builder = SSTableBuilder::new(&mut writer);
        while let Some(KV(key, value)) = self.heap.pop() {
            builder.write_ordered(&key, value);
        }
        builder.finish().unwrap();
    }
}

impl<T> EmitFn<KV<String, T>> for SSTableSink<T>
where
    T: PlumeTrait + Default,
{
    fn emit(&mut self, value: KV<String, T>) {
        self.heap.push(value);

        if self.heap.len() > MAX_SSTABLE_HEAP_SIZE {
            self.flush();
        }
    }

    fn finish(mut self: Box<Self>) {
        self.flush();
        let outputs: Vec<_> = self
            .configs
            .iter()
            .map(|c| c.get_filenames())
            .flatten()
            .map(|f| f.to_string())
            .collect();

        reshard(&self.sstables_written, &outputs);

        let mut pcoll_write = PCOLLECTION_REGISTRY.write().unwrap();
        let mut config = pcoll_write.get_mut(&self.configs[0].get_id()).unwrap();
        config.set_is_ptable(true);
        config.set_format(DataFormat::SSTABLE);
        config.set_num_resolved_shards(config.get_num_resolved_shards() + 1);
        if config.get_num_resolved_shards() == config.get_num_shards() {
            config.set_resolved(true);
        }
    }
}

struct MemorySink<T> {
    outputs: Vec<MemorySinkSingleOutput<T>>,
    write_count: usize,
}

struct OrderedMemorySink<T> {
    outputs: Vec<OrderedMemorySinkSingleOutput<T>>,
    write_count: usize,
}

impl<T> EmitFn<KV<String, T>> for OrderedMemorySink<T>
where
    T: PlumeTrait + Default,
{
    fn emit(&mut self, value: KV<String, T>) {
        let idx = self.write_count % self.outputs.len();
        self.outputs[idx].emit(value);
        self.write_count += 1;

        if self.write_count > IN_MEMORY_RECORD_THRESHOLD
            && self.write_count % IN_MEMORY_RECORD_THRESHOLD == 1
        {
            println!(
                "warning: ordered memory sink for {} got {} elements, might OOM",
                std::any::type_name::<T>().split("::").last().unwrap(),
                self.write_count
            );
        }
    }

    fn finish(self: Box<Self>) {
        for out in self.outputs {
            out.finish();
        }
    }
}

impl<T> OrderedMemorySink<T>
where
    T: PlumeTrait + Default,
{
    pub fn new(outputs: &[PCollectionProto]) -> Self {
        Self {
            outputs: outputs
                .iter()
                .map(|x| OrderedMemorySinkSingleOutput::new(x))
                .collect(),
            write_count: 0,
        }
    }
}

impl<T> EmitFn<T> for MemorySink<T>
where
    T: PlumeTrait + Default,
{
    fn finish(self: Box<Self>) {
        for out in self.outputs {
            out.finish();
        }
    }

    fn emit(&mut self, value: T) {
        let idx = self.write_count % self.outputs.len();
        self.outputs[idx].emit(value);
        self.write_count += 1;

        if self.write_count > IN_MEMORY_RECORD_THRESHOLD
            && self.write_count % IN_MEMORY_RECORD_THRESHOLD == 1
        {
            println!(
                "warning: memory sink for {} got {} elements, might OOM",
                std::any::type_name::<T>().split("::").last().unwrap(),
                self.write_count
            );
        }
    }
}

impl<T> MemorySink<T>
where
    T: PlumeTrait + Default,
{
    pub fn new(outputs: &[PCollectionProto]) -> Self {
        Self {
            outputs: outputs
                .iter()
                .map(|x| MemorySinkSingleOutput::new(x))
                .collect(),
            write_count: 0,
        }
    }
}

struct OrderedMemorySinkSingleOutput<T> {
    config: PCollectionProto,
    output: MinHeap<KV<String, T>>,
}

impl<T> OrderedMemorySinkSingleOutput<T>
where
    T: PlumeTrait + Default,
{
    fn new(config: &PCollectionProto) -> Self {
        Self {
            output: MinHeap::new(),
            config: config.clone(),
        }
    }

    fn emit(&mut self, value: KV<String, T>) {
        self.output.push(value);
    }

    pub fn finish(mut self) {
        let id = reserve_id();

        let output_vec: Vec<_> = self.output.into_iter_sorted().collect();

        IN_MEMORY_DATASETS
            .write()
            .unwrap()
            .insert(id, InMemoryPCollection::from_table(output_vec));

        let mut pcoll_write = PCOLLECTION_REGISTRY.write().unwrap();
        let mut config = pcoll_write.get_mut(&self.config.get_id()).unwrap();
        config.mut_memory_ids().push(id);
        config.set_format(self.config.get_format());
        config.set_is_ptable(true);
        config.set_num_resolved_shards(config.get_num_resolved_shards() + 1);
        if config.get_num_resolved_shards() == config.get_num_shards() {
            config.set_resolved(true);
        }
    }
}

struct MemorySinkSingleOutput<T> {
    config: PCollectionProto,
    output: Vec<T>,
}

impl<T> MemorySinkSingleOutput<T>
where
    T: PlumeTrait + Default,
{
    pub fn new(config: &PCollectionProto) -> Self {
        Self {
            config: config.clone(),
            output: Vec::new(),
        }
    }

    pub fn emit(&mut self, value: T) {
        self.output.push(value);
    }

    pub fn finish(mut self) {
        let id = reserve_id();
        println!("finished (regular)");

        IN_MEMORY_DATASETS
            .write()
            .unwrap()
            .insert(id, InMemoryPCollection::from_vec(self.output));

        let mut pcoll_write = PCOLLECTION_REGISTRY.write().unwrap();
        let mut config = pcoll_write.get_mut(&self.config.get_id()).unwrap();
        config.mut_memory_ids().push(id);
        config.set_format(self.config.get_format());

        config.set_num_resolved_shards(config.get_num_resolved_shards() + 1);
        if config.get_num_resolved_shards() == config.get_num_shards() {
            config.set_resolved(true);
        }
    }
}

struct Source<T> {
    config: PCollectionProto,
    _marker: std::marker::PhantomData<T>,
}

struct InMemorySourceIteratorSpec<T> {
    data: Arc<Vec<T>>,
    index: usize,
    end_index: usize,
}

struct InMemorySourceIteratorWrapper<T> {
    specs: Vec<InMemorySourceIteratorSpec<T>>,
}

struct InMemorySourceIterator<'a, T> {
    specs: &'a Vec<InMemorySourceIteratorSpec<T>>,
    spec_index: usize,
    index: usize,
}

struct InMemoryTableSourceIteratorSpec<T> {
    data: Arc<Vec<KV<String, T>>>,
}

struct InMemoryTableSourceIteratorWrapper<T> {
    specs: Vec<InMemoryTableSourceIteratorSpec<T>>,
    start_key: String,
    end_key: String,
}

struct InMemoryTableSourceIterator<'a, T> {
    iters: Vec<std::slice::Iter<'a, KV<String, T>>>,
    heap: MinHeap<KV<&'a KV<String, T>, usize>>,
}

impl<T> Source<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(config: PCollectionProto) -> Self {
        Self {
            config: config,
            _marker: std::marker::PhantomData {},
        }
    }

    pub fn mem_source<'a>(&'a self) -> InMemorySourceIteratorWrapper<T> {
        let mut output = InMemorySourceIteratorWrapper { specs: Vec::new() };

        for memory_id in self.config.get_memory_ids() {
            let data = {
                let guard = IN_MEMORY_DATASETS.read().unwrap();
                let dataset = guard
                    .get(&memory_id)
                    .expect(&format!("Failed to look up data in id={}", &memory_id));
                let pcoll: Arc<dyn InMemoryPCollectionWrapper> = dataset.data.clone();
                let data: &InMemoryPCollectionUnderlying<T> =
                    pcoll.downcast_ref().expect("failed to downcast!");
                data.data.clone()
            };
            output.specs.push(InMemorySourceIteratorSpec {
                index: self.config.get_starting_index() as usize,
                end_index: self.config.get_ending_index() as usize,
                data: data,
            })
        }
        output
    }
}

trait MakeKVReader<T> {
    fn make(config: &PCollectionProto) -> Box<dyn StreamingIterator<Item = T>>;
}

impl<T> MakeKVReader<T> for Source<T> {
    default fn make(config: &PCollectionProto) -> Box<dyn StreamingIterator<Item = T>> {
        panic!("can't make sstable reader with unknown type T")
    }
}

impl<T> MakeKVReader<KV<String, T>> for Source<KV<String, T>>
where
    T: PlumeTrait + Default,
{
    default fn make(config: &PCollectionProto) -> Box<dyn StreamingIterator<Item = KV<String, T>>> {
        let reader = ShardedSSTableReader::from_filenames(
            config.get_filenames(),
            config.get_starting_key(),
            config.get_ending_key().to_string(),
        )
        .unwrap();

        Box::new(reader)
    }
}

impl<T> Source<T>
where
    T: PlumeTrait + Default,
{
    pub fn sstable_source_or_panic(&self) -> Box<dyn StreamingIterator<Item = T>> {
        Self::make(&self.config)
    }

    pub fn recordio_source(&self) -> recordio::RecordIOReaderOwned<T> {
        let filenames = self.config.get_filenames();
        assert!(
            filenames.len() == 1,
            "I don't know how to read from multiple recordios at once!"
        );
        let mut f = std::fs::File::open(&filenames[0]).unwrap();

        recordio::RecordIOReaderOwned::new(Box::new(std::io::BufReader::new(f)))
    }
}

impl<T> Source<KV<String, T>>
where
    T: PlumeTrait + Default,
{
    pub fn sstable_source(&self) -> ShardedSSTableReader<T> {
        ShardedSSTableReader::from_filenames(
            self.config.get_filenames(),
            self.config.get_starting_key(),
            self.config.get_ending_key().to_string(),
        )
        .unwrap()
    }

    pub fn mem_table_source<'a>(&'a self) -> InMemoryTableSourceIteratorWrapper<T> {
        let mut output = InMemoryTableSourceIteratorWrapper {
            specs: Vec::new(),
            start_key: self.config.get_starting_key().to_string(),
            end_key: self.config.get_ending_key().to_string(),
        };

        for memory_id in self.config.get_memory_ids() {
            let data = {
                let guard = IN_MEMORY_DATASETS.read().unwrap();
                let dataset = guard
                    .get(&memory_id)
                    .expect(&format!("Failed to look up data in id={}", &memory_id));
                let pcoll: Arc<dyn InMemoryPCollectionWrapper> = dataset.data.clone();
                let data: &InMemoryPCollectionUnderlying<KV<String, T>> =
                    pcoll.downcast_ref().expect("failed to downcast!");
                data.data.clone()
            };
            output
                .specs
                .push(InMemoryTableSourceIteratorSpec { data: data })
        }
        output
    }
}

impl<'b, T> Source<KV<String, Stream<'b, T>>>
where
    T: PlumeTrait + Default,
{
    pub fn sstable_source(&self) -> ShardedSSTableReader<T> {
        ShardedSSTableReader::from_filenames(
            self.config.get_filenames(),
            self.config.get_starting_key(),
            self.config.get_ending_key().to_string(),
        )
        .unwrap()
    }

    pub fn mem_table_grouped_source<'a>(&'a self) -> InMemoryTableSourceIteratorWrapper<T> {
        let mut output = InMemoryTableSourceIteratorWrapper {
            specs: Vec::new(),
            start_key: self.config.get_starting_key().to_string(),
            end_key: self.config.get_ending_key().to_string(),
        };
        for memory_id in self.config.get_memory_ids() {
            let data = {
                let guard = IN_MEMORY_DATASETS.read().unwrap();
                let dataset = guard
                    .get(&memory_id)
                    .expect(&format!("Failed to look up data in id={}", &memory_id));
                let pcoll: Arc<dyn InMemoryPCollectionWrapper> = dataset.data.clone();
                let data: &InMemoryPCollectionUnderlying<KV<String, T>> =
                    pcoll.downcast_ref().expect("failed to downcast!");
                data.data.clone()
            };
            output
                .specs
                .push(InMemoryTableSourceIteratorSpec { data: data })
        }
        output
    }
}

impl<T> InMemoryTableSourceIteratorWrapper<T> {
    pub fn empty() -> InMemoryTableSourceIteratorWrapper<T> {
        InMemoryTableSourceIteratorWrapper {
            specs: Vec::new(),
            start_key: String::new(),
            end_key: String::new(),
        }
    }

    pub fn iter<'a>(&'a self) -> InMemoryTableSourceIterator<'a, T> {
        let mut iterator = InMemoryTableSourceIterator {
            iters: self
                .specs
                .iter()
                .map(|x| {
                    let start_bound = if self.start_key.is_empty() {
                        0
                    } else {
                        x.data
                            .binary_search_by(|kv| {
                                if &kv.key().as_str() >= &self.start_key.as_str() {
                                    return Ordering::Greater;
                                }
                                Ordering::Less
                            })
                            .unwrap_or_else(|e| e)
                    };

                    let end_bound = if self.end_key.is_empty() {
                        x.data.len()
                    } else {
                        x.data
                            .binary_search_by(|kv| {
                                if &kv.key().as_str() >= &self.end_key.as_str() {
                                    return Ordering::Greater;
                                }
                                Ordering::Less
                            })
                            .unwrap_or_else(|e| e)
                    };

                    x.data[start_bound..end_bound].iter()
                })
                .collect(),
            heap: MinHeap::new(),
        };

        for (idx, iter) in iterator.iters.iter_mut().enumerate() {
            if let Some(kv) = iter.next() {
                iterator.heap.push(KV::new(kv, idx));
            }
        }

        iterator
    }
}

impl<T> InMemorySourceIteratorWrapper<T> {
    pub fn iter<'a>(&'a self) -> InMemorySourceIterator<'a, T> {
        let index = if self.specs.len() > 0 {
            self.specs[0].index
        } else {
            0
        };

        InMemorySourceIterator {
            spec_index: 0,
            index: index,
            specs: &self.specs,
        }
    }
}

impl<'b, T> StreamingIterator for InMemorySourceIterator<'b, T> {
    type Item = T;

    fn peek<'a>(&'a mut self) -> Option<&'a Self::Item> {
        if self.spec_index >= self.specs.len() {
            return None;
        }

        let spec = &self.specs[self.spec_index];
        let result = Some(&spec.data[self.index]);
        return result;
    }

    fn next<'a>(&'a mut self) -> Option<&'a Self::Item> {
        loop {
            if self.spec_index >= self.specs.len() {
                return None;
            }

            let spec = &self.specs[self.spec_index];
            if spec.end_index > 0 && self.index >= spec.end_index || self.index >= spec.data.len() {
                self.spec_index += 1;
                if self.spec_index >= self.specs.len() {
                    return None;
                }
                self.index = self.specs[self.spec_index].index;
                continue;
            }

            let result = Some(&spec.data[self.index]);
            self.index += 1;
            return result;
        }
    }
}

impl<'a, T> StreamingIterator for InMemoryTableSourceIterator<'a, T> {
    type Item = KV<String, T>;

    fn peek(&mut self) -> Option<&Self::Item> {
        if let Some(kv) = self.heap.peek() {
            let idx = kv.1;
            let output = kv.0;

            return Some(&output);
        }

        None
    }

    fn next(&mut self) -> Option<&Self::Item> {
        if let Some(kv) = self.heap.pop() {
            let idx = *kv.value();
            let output = kv.key();

            if let Some(kv) = self.iters[idx].next() {
                self.heap.push(KV::new(kv, idx));
            }

            return Some(&output);
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_recordio() {
        let mut input = PCollectionProto::new();
        input.set_format(DataFormat::RECORDIO);
        input
            .mut_filenames()
            .push(String::from("/tmp/data.recordio@2"));
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&[input], 10)
                .iter()
                .map(|x| x[0].get_filenames()[0].clone())
                .collect::<Vec<_>>(),
            vec![
                "/tmp/data.recordio-00000-of-00002",
                "/tmp/data.recordio-00001-of-00002",
            ]
        );
    }

    #[test]
    fn test_shard_in_memory() {
        let p = PCollection::<Primitive<u64>>::from_primitive_vec(vec![1, 2, 3, 4]);
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&[p.underlying.proto.clone()], 10)
                .iter()
                .map(|x| (x[0].get_starting_index(), x[0].get_ending_index()))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2), (2, 3), (3, 0)]
        );
    }

    #[test]
    fn test_shard_in_memory_2() {
        let p =
            PCollection::<Primitive<u64>>::from_primitive_vec(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&[p.underlying.proto.clone()], 2)
                .iter()
                .map(|x| (x[0].get_starting_index(), x[0].get_ending_index()))
                .collect::<Vec<_>>(),
            vec![(0, 5), (5, 0)]
        );
    }

    #[test]
    fn test_shard_in_memory_3() {
        let p = PCollection::<Primitive<u64>>::from_primitive_vecs(vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ]);
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&[p.underlying.proto.clone()], 2)
                .iter()
                .map(|x| (x[0].get_memory_ids().len()))
                .collect::<Vec<_>>(),
            vec![2, 1]
        );
    }

    fn pcoll_to_string(s: &PCollectionProto) -> String {
        format!(
            "{:?} ({} memory locations), idx {}-->{}",
            s.get_format(),
            s.get_memory_ids().len(),
            s.get_starting_index(),
            s.get_ending_index()
        )
    }

    fn pcoll(format: DataFormat, start: usize, end: usize, memory_ids: usize) -> String {
        let mut s = PCollectionProto::new();
        s.set_format(format);
        s.set_starting_index(start as u64);
        s.set_ending_index(end as u64);
        for i in 0..memory_ids {
            s.mut_memory_ids().push(i as u64);
        }
        pcoll_to_string(&s)
    }

    #[test]
    fn test_planning() {
        let mut planner = Planner::new();
        planner.target_shards = 10;

        let mut stage = Stage::new();
        let p_in = PCollection::<Primitive<u64>>::from_primitive_vecs(vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ]);
        stage.mut_inputs().push(p_in.underlying.proto.clone());

        let p_out =
            PCollection::<KV<String, Primitive<u64>>>::from_sstable("/data/output.sstable@5");
        stage.mut_outputs().push(p_out.underlying.proto.clone());

        let shards = planner.plan(&stage);

        assert_eq!(
            shards[0]
                .get_inputs()
                .iter()
                .map(|i| pcoll_to_string(i))
                .collect::<Vec<_>>(),
            vec![pcoll(DataFormat::IN_MEMORY, 0, 5, 1)]
        );

        assert_eq!(
            shards[0]
                .get_outputs()
                .iter()
                .map(|i| pcoll_to_string(i))
                .collect::<Vec<_>>(),
            vec![pcoll(DataFormat::SSTABLE, 0, 0, 0)]
        );
    }
}
