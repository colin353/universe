extern crate plume_proto_rust;
use plume_proto_rust::*;

#[macro_use]
extern crate lazy_static;

use std::collections::HashMap;
use std::rc::Rc;
use std::sync::RwLock;

lazy_static! {
    static ref REGISTRY: RwLock<HashMap<u64, String>> = { RwLock::new(HashMap::new()) };
}

pub struct PCollection<T> {
    dependency: Option<Box<dyn PFn>>,
    _marker: std::marker::PhantomData<T>,
}

pub type PTable<T1, T2> = PCollection<(T1, T2)>;

impl<T> PCollection<T>
where
    T: 'static,
{
    pub fn new() -> Self {
        Self {
            dependency: None,
            _marker: std::marker::PhantomData {},
        }
    }

    fn render(&self) -> String {
        format!("[PCollection<?>]")
    }

    pub fn par_do<O, DoType>(self, f: DoType) -> PCollection<O>
    where
        DoType: DoFn<Input = T, Output = O> + 'static,
        O: 'static,
    {
        let mut out: PCollection<O> = PCollection::<O>::new();
        out.dependency = Some(Box::new(DoFnWrapper {
            dependency: Rc::new(self),
            function: Box::new(f),
        }));
        out
    }
}

impl<K, V> PCollection<(K, V)>
where
    K: 'static,
    V: 'static,
{
    pub fn is_ptable(&self) -> bool {
        true
    }

    pub fn join<V2, O, JoinType>(self, right: PTable<K, V2>, f: JoinType) -> PCollection<O>
    where
        JoinType: JoinFn<Key = K, ValueLeft = V, ValueRight = V2, Output = O> + 'static,
        V2: 'static,
        O: 'static,
    {
        let mut out = PCollection::<O>::new();
        out.dependency = Some(Box::new(JoinFnWrapper {
            dependency_left: Rc::new(self),
            dependency_right: Rc::new(right),
            function: Box::new(f),
        }));
        out
    }
}

pub fn compute<T>(input: PCollection<T>) -> Vec<T> {
    println!("[start]");
    if let Some(x) = input.dependency {
        println!("|");
        println!("V");
        println!("{}", x.render());
    }
    println!("|");
    println!("V");
    println!("[PCollection]");

    Vec::new()
}

pub trait PFn {
    fn render(&self) -> String;
}
pub trait EmitFn<T> {
    fn emit(&mut self, value: T);
}

pub struct JoinFnWrapper<K, V1, V2, O> {
    dependency_left: Rc<PCollection<(K, V1)>>,
    dependency_right: Rc<PCollection<(K, V2)>>,
    function: Box<JoinFn<Key = K, ValueLeft = V1, ValueRight = V2, Output = O>>,
}

pub struct Stream<T> {
    _mark: std::marker::PhantomData<T>,
}

pub trait JoinFn {
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
    dependency: Rc<PCollection<T1>>,
    function: Box<DoFn<Input = T1, Output = T2>>,
}
pub trait DoFn {
    type Input;
    type Output;
    fn do_it(&self, input: Self::Input, emit: &mut dyn EmitFn<Self::Output>);
}
impl<T1, T2> PFn for DoFnWrapper<T1, T2> {
    fn render(&self) -> String {
        let mut output = String::new();
        if let Some(ref x) = self.dependency.dependency {
            output = format!("{}\n|\nV\n", x.render());
        }
        output += &format!("[PCollection]\n|\nV\n[DoFn]\n");
        output
    }
}

impl<K, V1, V2, O> PFn for JoinFnWrapper<K, V1, V2, O> {
    fn render(&self) -> String {
        let mut left = String::new();
        if let Some(ref x) = self.dependency_left.dependency {
            left = x.render();
            left += "|\nV\n";
        }
        left += "[PCollection]\n";

        let mut right = String::new();
        if let Some(ref x) = self.dependency_right.dependency {
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
