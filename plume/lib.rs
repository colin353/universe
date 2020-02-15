extern crate plume_proto_rust;
extern crate shard_lib;

#[macro_use]
extern crate lazy_static;

use plume_proto_rust::*;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::sync::RwLock;

static ORDER: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;
static LAST_ID: AtomicU64 = AtomicU64::new(1);

static TARGET_SHARDS: usize = 10;
static IN_MEMORY_RECORD_THRESHOLD: u64 = 1000 * 1000;
static IN_MEMORY_BYTES_THRESHOLD: u64 = 1000 * 1000 * 1000;

lazy_static! {
    static ref PCOLLECTION_REGISTRY: RwLock<HashMap<u64, PCollectionProto>> =
        { RwLock::new(HashMap::new()) };
    static ref PFN_REGISTRY: RwLock<HashMap<u64, Arc<dyn PFn>>> = { RwLock::new(HashMap::new()) };
    static ref IN_MEMORY_DATASETS: RwLock<HashMap<u64, InMemoryPCollection>> =
        { RwLock::new(HashMap::new()) };
}

fn reserve_id() -> u64 {
    LAST_ID.fetch_add(1, ORDER)
}

pub trait InMemoryPCollectionWrapper: Send + Sync {
    fn len(&self) -> usize;
}

pub struct InMemoryPCollectionUnderlying<T> {
    data: RwLock<Vec<T>>,
}

impl<T> InMemoryPCollectionWrapper for InMemoryPCollectionUnderlying<T>
where
    T: Send + Sync,
{
    fn len(&self) -> usize {
        self.data.read().unwrap().len()
    }
}

pub struct InMemoryPCollection {
    data: Arc<Box<dyn InMemoryPCollectionWrapper>>,
}

impl InMemoryPCollection {
    pub fn from_vec<T>(data: Vec<T>) -> Self
    where
        T: Send + Sync + 'static,
    {
        Self {
            data: Arc::new(Box::new(InMemoryPCollectionUnderlying {
                data: RwLock::new(data),
            })),
        }
    }

    fn len(&self) -> usize {
        self.data.len()
    }
}

pub struct PCollection<T> {
    underlying: Arc<PCollectionUnderlying<T>>,
}

pub type PTable<T1, T2> = PCollection<(T1, T2)>;

pub struct PCollectionUnderlying<T> {
    id: AtomicU64,
    dependency: Option<Arc<dyn PFn>>,
    proto: PCollectionProto,
    _marker: std::marker::PhantomData<T>,
}

impl<T> PCollection<T>
where
    T: 'static + Send + Sync,
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
        config.set_filename(filename.to_string());
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
        O: 'static + Send + Sync,
    {
        let mut out: PCollection<O> = PCollection::<O>::new();
        Arc::get_mut(&mut out.underlying).unwrap().dependency = Some(Arc::new(DoFnWrapper {
            dependency: self.clone(),
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
}

impl<K, V> PCollection<(K, V)>
where
    K: 'static + Send + Sync,
    V: 'static + Send + Sync,
{
    pub fn join<V2, O, JoinType>(self, right: PTable<K, V2>, f: JoinType) -> PCollection<O>
    where
        JoinType: JoinFn<Key = K, ValueLeft = V, ValueRight = V2, Output = O> + 'static,
        V2: 'static + Send + Sync,
        O: 'static + Send + Sync,
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

impl<String, V> PCollection<(String, V)>
where
    V: 'static + Send + Sync,
{
    pub fn from_sstable(filename: &str) -> Self {
        let mut config = PCollectionProto::new();
        config.set_filename(filename.to_string());
        config.set_resolved(true);
        config.set_format(DataFormat::SSTABLE);

        PCollection {
            underlying: Arc::new(PCollectionUnderlying::<(String, V)> {
                id: AtomicU64::new(0),
                dependency: None,
                proto: config,
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    pub fn group_by_key(&self) -> PCollection<(String, Stream<V>)> {
        let mut config = self.underlying.proto.clone();
        config.set_group_by_key(true);

        PCollection {
            underlying: Arc::new(PCollectionUnderlying::<(String, Stream<V>)> {
                id: AtomicU64::new(0),
                dependency: self.underlying.dependency.clone(),
                proto: config,
                _marker: std::marker::PhantomData {},
            }),
        }
    }
}

pub fn update_stage(stage: &mut Stage) {
    for input in stage.mut_inputs().iter_mut() {
        let reg = PCOLLECTION_REGISTRY.read().unwrap();
        let latest_input = reg.get(&input.get_id()).unwrap();

        *input = latest_input.clone();
    }
}

pub fn run<T>(input: PCollection<T>)
where
    T: 'static + Send + Sync,
{
    let mut stages = input.stages();
    let mut completed = std::collections::HashSet::new();
    loop {
        let mut did_execute = false;
        for (id, stage) in stages.iter_mut().enumerate() {
            update_stage(stage);

            if completed.contains(&id) {
                continue;
            }

            let mut ready = true;
            for input in stage.get_inputs() {
                if !input.get_resolved() {
                    ready = false;
                }
            }

            if ready {
                execute_stage(stage);
                did_execute = true;
                completed.insert(id);
            }
        }
        if !did_execute {
            break;
        }
    }

    if completed.len() != stages.len() {
        panic!("Deadlock, didn't execute all stages");
    }
}

pub fn execute_stage(stage: &Stage) {
    println!("Executing stage");

    let pfn = PFN_REGISTRY
        .read()
        .unwrap()
        .get(&stage.get_function().get_id())
        .unwrap()
        .clone();

    let mut result = pfn.execute(stage);

    // Update the outputs so they are "resolved"
    for output in result.take_outputs().into_iter() {
        PCOLLECTION_REGISTRY
            .write()
            .unwrap()
            .insert(output.get_id(), output);
    }
}

pub trait PFn: Send + Sync {
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>);
    fn execute(&self, stage: &Stage) -> Stage;
}
pub trait EmitFn<T> {
    fn emit(&mut self, value: T);
}

pub struct JoinFnWrapper<K, V1, V2, O> {
    dependency_left: Arc<PCollection<(K, V1)>>,
    dependency_right: Arc<PCollection<(K, V2)>>,
    function: Box<JoinFn<Key = K, ValueLeft = V1, ValueRight = V2, Output = O>>,
}

pub struct Stream<T> {
    _mark: std::marker::PhantomData<T>,
}

pub trait JoinFn: Send + Sync {
    type Key;
    type ValueLeft;
    type ValueRight;
    type Output;
    fn join(
        &self,
        key: Self::Key,
        left: Stream<Self::ValueLeft>,
        right: Stream<Self::ValueRight>,
        emit: &mut dyn EmitFn<Self::Output>,
    );
}

pub struct DoFnWrapper<T1, T2> {
    dependency: PCollection<T1>,
    function: Box<DoFn<Input = T1, Output = T2>>,
}
pub trait DoFn: Send + Sync {
    type Input;
    type Output;
    fn do_it(&self, input: Self::Input, emit: &mut dyn EmitFn<Self::Output>);
}
impl<T1, T2> PFn for DoFnWrapper<T1, T2>
where
    T1: 'static + Send + Sync,
    T2: 'static + Send + Sync,
{
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>) {
        let dep_stages = self.dependency.stages();

        let mut s = Stage::new();
        s.mut_inputs().push(self.dependency.to_proto());
        s.mut_function().set_description(String::from("DoFn"));
        s.mut_function().set_id(id);

        (s, dep_stages)
    }

    fn execute(&self, stage: &Stage) -> Stage {
        let mut output = stage.clone();
        output.clear_outputs();
        for out in stage.get_outputs() {
            let mut resolved_output = out.clone();
            resolved_output.set_resolved(true);
            output.mut_outputs().push(resolved_output);
        }

        output
    }
}

impl<K, V1, V2, O> PFn for JoinFnWrapper<K, V1, V2, O>
where
    K: 'static + Send + Sync,
    V1: 'static + Send + Sync,
    V2: 'static + Send + Sync,
    O: 'static + Send + Sync,
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

    fn execute(&self, stage: &Stage) -> Stage {
        let mut output = stage.clone();
        output.clear_outputs();
        for out in stage.get_outputs() {
            let mut resolved_output = out.clone();
            resolved_output.set_resolved(true);
            output.mut_outputs().push(resolved_output);
        }

        output
    }
}

struct Planner {
    target_shards: usize,
    in_memory_record_threshold: u64,
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

        if stage.get_inputs().len() != 1 {
            panic!(
                "I don't know how to plan for {} inputs",
                stage.get_inputs().len()
            );
        }

        let input = &stage.get_inputs()[0];

        if output.get_format() == DataFormat::UNKNOWN {
            // If the output is not defined, we should decide what to use. If the
            // data is not too big, we will keep it in memory, else write to disk
            let size = Self::estimate_size(&input);
            if self.keep_in_memory(size) {
                output.set_format(DataFormat::IN_MEMORY);
            } else {
                output.set_format(DataFormat::SSTABLE);
            }
        }

        let sharded_inputs = self.shard_inputs(&input, target_shards);
        let sharded_outputs = self.shard_output(&output, target_shards);

        if sharded_inputs.len() != sharded_outputs.len() {
            panic!(
                "Can't plan: got {} inputs and {} outputs!",
                sharded_inputs.len(),
                sharded_outputs.len()
            );
        }

        for (shard_input, shard_output) in
            sharded_inputs.into_iter().zip(sharded_outputs.into_iter())
        {
            let mut shard = Shard::new();
            shard.mut_inputs().push(shard_input);
            shard.set_function(stage.get_function().clone());
            shard.mut_outputs().push(shard_output);

            shards.push(shard);
        }

        shards
    }

    fn shard_output(
        &self,
        output: &PCollectionProto,
        target_shards: usize,
    ) -> Vec<PCollectionProto> {
        let mut output = output.clone();

        let mut shards = Vec::new();
        if output.get_format() == DataFormat::IN_MEMORY {
            for index in 0..target_shards {
                let mut s = output.clone();
                shards.push(s);
            }
            return shards;
        }

        if output.get_format() == DataFormat::SSTABLE {
            let sharded_filename = output.get_filename().to_string();
            if sharded_filename.is_empty() {
                let sharded_filename = format!(
                    "{}/output{:02}.sstable@{}",
                    self.temp_data_folder,
                    output.get_id(),
                    target_shards
                );
            }
            for filename in shard_lib::unshard(&sharded_filename) {
                let mut s = output.clone();
                s.set_filename(filename);
                shards.push(s);
            }

            return shards;
        }

        if output.get_format() == DataFormat::RECORDIO {
            let sharded_filename = output.get_filename().to_string();
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
                s.set_filename(filename);
                shards.push(s);
            }

            return shards;
        }

        panic!(
            "I don't know how to shard output for type {:?}!",
            output.get_format()
        );
    }

    fn keep_in_memory(&self, size: SizeEstimate) -> bool {
        if size.get_records() > self.in_memory_record_threshold {
            return false;
        }

        if size.get_data_bytes() > self.in_memory_bytes_threshold {
            return false;
        }

        true
    }

    pub fn estimate_size(input: &PCollectionProto) -> SizeEstimate {
        let mut out = SizeEstimate::new();
        if input.get_format() == DataFormat::IN_MEMORY {
            let mut count = 0;
            for memory_id in input.get_memory_ids() {
                let mem_reader = IN_MEMORY_DATASETS.read().unwrap();
                count += mem_reader.get(memory_id).unwrap().len();
            }
            out.set_records(count as u64);
        }
        out
    }

    pub fn count_shards(input: &PCollectionProto) -> Option<usize> {
        if input.get_format() == DataFormat::IN_MEMORY {
            return Some(input.get_memory_ids().len());
        }

        if input.get_format() == DataFormat::RECORDIO || input.get_format() == DataFormat::SSTABLE {
            if input.get_filename().is_empty() {
                return None;
            }
            return Some(shard_lib::unshard(input.get_filename()).len());
        }

        None
    }

    fn shard_inputs(
        &self,
        input: &PCollectionProto,
        target_shards: usize,
    ) -> Vec<PCollectionProto> {
        // RecordIO doesn't support keyrange sharding, so we have to just use whatever sharding
        // strategy was present on the input.
        if input.get_format() == DataFormat::RECORDIO {
            return shard_lib::unshard(input.get_filename())
                .iter()
                .map(|f| {
                    let mut s = input.clone();
                    s.set_filename(f.to_string());
                    s
                })
                .collect();
        }

        if input.get_format() == DataFormat::IN_MEMORY {
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
                return output;
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
                output.push(s);
            }

            return output;
        }

        Vec::new()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_recordio() {
        let mut input = PCollectionProto::new();
        input.set_format(DataFormat::RECORDIO);
        input.set_filename(String::from("/tmp/data.recordio@2"));
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&input, 10)
                .iter()
                .map(|x| x.get_filename())
                .collect::<Vec<_>>(),
            vec![
                "/tmp/data.recordio-00000-of-00002",
                "/tmp/data.recordio-00001-of-00002",
            ]
        );
    }

    #[test]
    fn test_shard_in_memory() {
        let p = PCollection::<u64>::from_vec(vec![1, 2, 3, 4]);
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&p.underlying.proto, 10)
                .iter()
                .map(|x| (x.get_starting_index(), x.get_ending_index()))
                .collect::<Vec<_>>(),
            vec![(0, 1), (1, 2), (2, 3), (3, 0)]
        );
    }

    #[test]
    fn test_shard_in_memory_2() {
        let p = PCollection::<u64>::from_vec(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&p.underlying.proto, 2)
                .iter()
                .map(|x| (x.get_starting_index(), x.get_ending_index()))
                .collect::<Vec<_>>(),
            vec![(0, 5), (5, 0)]
        );
    }

    #[test]
    fn test_shard_in_memory_3() {
        let p = PCollection::<u64>::from_vecs(vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ]);
        let planner = Planner::new();
        assert_eq!(
            planner
                .shard_inputs(&p.underlying.proto, 2)
                .iter()
                .map(|x| (x.get_memory_ids().len()))
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
        let p_in = PCollection::<u64>::from_vecs(vec![
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
        ]);
        stage.mut_inputs().push(p_in.underlying.proto.clone());

        let p_out = PCollection::<(String, u64)>::from_sstable("/data/output.sstable@5");
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
