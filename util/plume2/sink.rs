use crate::{globals, reserve_id, EmitFn, InMemoryCollection};
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
            return Box::new(join(outputs.iter().map(|c| MemorySink::new(c)).collect()));
        }
        panic!(
            "I don't know how to make a sink for {:?}",
            outputs[0].format
        );
    }
}

pub struct JoinedSink<E> {
    outputs: Vec<E>,
    write_count: usize,
}

impl<T, E: EmitFn<T>> EmitFn<T> for JoinedSink<E> {
    fn finish(self: Box<Self>) {
        for out in self.outputs {
            Box::new(out).finish();
        }
    }

    fn emit(&mut self, value: T) {
        let len = self.outputs.len();
        self.outputs[self.write_count % len].emit(value);
    }
}

pub fn join<T, E: EmitFn<T>>(outputs: Vec<E>) -> JoinedSink<E> {
    JoinedSink {
        outputs,
        write_count: 0,
    }
}

struct MemorySink<T> {
    id: u64,
    output: Vec<T>,
}

impl<T> MemorySink<T> {
    pub fn new(config: &CollectionSpec) -> Self {
        Self {
            id: config.id,
            output: Vec::new(),
        }
    }
}

impl<T: Send + Sync + 'static> EmitFn<T> for MemorySink<T> {
    fn finish(self: Box<Self>) {
        let shard_id = reserve_id();
        globals::IN_MEMORY_DATASETS
            .write()
            .unwrap()
            .insert(shard_id, InMemoryCollection::from_vec(self.output));

        let mut guard = globals::COLLECTION_REGISTRY.write().unwrap();
        let mut config = guard
            .get_mut(&self.id)
            .expect("failed to find collection spec in global registry!");
        config.memory_ids.push(shard_id);
        config.format = DataFormat::InMemory;
        config.num_resolved_shards += 1;
        if config.num_resolved_shards == config.num_shards {
            config.resolved = true;
        }
    }

    fn emit(&mut self, value: T) {
        self.output.push(value);
    }
}
