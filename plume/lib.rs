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
        let mut out: PCollection<O> = PCollection::new();
        out.dependency = Some(Box::new(DoFnWrapper {
            dependency: Rc::new(self),
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

pub struct DoFnWrapper<T1, T2> {
    dependency: Rc<PCollection<T1>>,
    function: Box<DoFn<Input = T1, Output = T2>>,
}
pub trait DoFn {
    type Input;
    type Output;
    fn do_it(&self, input: &Self::Input, emit: &mut dyn EmitFn<Self::Output>);
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
