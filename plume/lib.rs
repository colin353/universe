extern crate plume_proto_rust;
use plume_proto_rust::*;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;
use std::sync::RwLock;

static ORDER: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;
static LAST_ID: AtomicU64 = AtomicU64::new(1);

lazy_static! {
    static ref PCOLLECTION_REGISTRY: RwLock<HashMap<u64, PCollectionProto>> =
        { RwLock::new(HashMap::new()) };
    static ref PFN_REGISTRY: RwLock<HashMap<u64, Arc<dyn PFn>>> = { RwLock::new(HashMap::new()) };
}

fn reserve_id() -> u64 {
    LAST_ID.fetch_add(1, ORDER)
}

pub struct PCollection<T> {
    underlying: Arc<PCollectionUnderlying<T>>,
}

pub type PTable<T1, T2> = PCollection<(T1, T2)>;

pub struct PCollectionUnderlying<T> {
    id: AtomicU64,
    dependency: Option<Arc<dyn PFn>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> PCollection<T>
where
    T: 'static + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            underlying: Arc::new(PCollectionUnderlying::<T> {
                id: AtomicU64::new(0),
                dependency: None,
                _marker: std::marker::PhantomData {},
            }),
        }
    }

    fn render(&self) -> String {
        format!("[PCollection<?>]")
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
        let mut out = PCollectionProto::new();
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
    pub fn is_ptable(&self) -> bool {
        true
    }

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

pub fn compute<T>(input: PCollection<T>) -> Vec<T> {
    println!("[start]");
    if let Some(ref x) = input.underlying.dependency {
        println!("|");
        println!("V");
        println!("{}", x.render());
    }
    println!("|");
    println!("V");
    println!("[PCollection]");

    Vec::new()
}

pub fn stages<T>(input: PCollection<T>) -> Vec<Stage>
where
    T: 'static + Send + Sync,
{
    input.stages()
}

pub trait PFn: Send + Sync {
    fn render(&self) -> String;
    fn stages(&self, id: u64) -> (Stage, Vec<Stage>);
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

    fn render(&self) -> String {
        let mut output = String::new();
        if let Some(ref x) = self.dependency.underlying.dependency {
            output = format!("{}\n|\nV\n", x.render());
        }
        output += &format!("[PCollection]\n|\nV\n[DoFn]\n");
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

    fn render(&self) -> String {
        let mut left = String::new();
        if let Some(ref x) = self.dependency_left.underlying.dependency {
            left = x.render();
            left += "|\nV\n";
        }
        left += "[PCollection]\n";

        let mut right = String::new();
        if let Some(ref x) = self.dependency_right.underlying.dependency {
            right = x.render();
            right += "|\nV\n";
        }
        right += "[PCollection]\n";
        let mut lines_left = left.lines();
        let mut lines_right = right.lines();
        let mut output = String::new();
        loop {
            let left = lines_left.next();
            let right = lines_right.next();
            if left.is_none() && right.is_none() {
                break;
            }
            output += &format!("{:30} {}\n", left.unwrap_or(""), right.unwrap_or(""));
        }
        output += "|\n|\nV\n[JoinFn]\n";

        output
    }
}
