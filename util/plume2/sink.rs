use crate::{Collection, EmitFn, PFn};
use plume_bus::*;

pub struct SinkProducer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> SinkProducer<T> {
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData {},
        }
    }
}

pub trait SinkProducerTrait<T> {
    fn make_sink(&self, outputs: &[CollectionSpec]) -> Box<dyn EmitFn<T>>;
}

impl<T> SinkProducerTrait<T> for SinkProducer<T>
where
    T: Send + Sync + 'static,
{
    default fn make_sink(&self, outputs: &[CollectionSpec]) -> Box<dyn EmitFn<T>> {
        if outputs.len() == 0 {
            panic!("Can't make a sink from zero outputs");
        }
        if outputs[0].format == DataFormat::InMemory {
            return Box::new(MemorySink::new(outputs));
        }
        panic!(
            "I don't know how to make a sink for {:?}",
            outputs[0].format
        );
    }
}

struct MemorySink<T> {
    outputs: Vec<MemorySinkSingleOutput<T>>,
    write_count: usize,
}

impl<T> MemorySink<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(outputs: &[CollectionSpec]) -> Self {
        Self {
            outputs: outputs
                .iter()
                .map(|x| MemorySinkSingleOutput::new(x))
                .collect(),
            write_count: 0,
        }
    }
}

impl<T> EmitFn<T> for MemorySink<T>
where
    T: Send + Sync + 'static,
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
    }
}

struct MemorySinkSingleOutput<T> {
    config: CollectionSpec,
    output: Vec<T>,
}
