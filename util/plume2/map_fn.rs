use crate::sink::{SinkProducer, SinkProducerTrait};
use crate::{get_collection, reserve_id, Collection, EmitFn, PFn};
use plume_bus::*;

pub struct MapFnWrapper<In, Out> {
    id: u64,
    dependency: u64,
    function: Box<dyn MapFn<Input = In, Output = Out>>,
}

impl<In, Out> MapFnWrapper<In, Out> {
    pub fn new(dependency: u64, function: Box<dyn MapFn<Input = In, Output = Out>>) -> Self {
        MapFnWrapper {
            id: reserve_id(),
            dependency,
            function,
        }
    }
}

pub trait MapFn: Send + Sync {
    type Input;
    type Output;
    fn map(&self, input: &Self::Input, emit: &mut dyn EmitFn<Self::Output>);

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>().split("::").last().unwrap()
    }
}

impl<In, Out> PFn for MapFnWrapper<In, Out>
where
    In: Send + Sync + 'static,
    Out: Send + Sync + 'static,
{
    fn plan(&self) -> (Plan, Vec<Plan>) {
        let input = get_collection(self.dependency);
        let dep_plans = Vec::new();
        let mut s = Plan::new();
        s.inputs.push(input);
        s.function.description = String::from("MapFn");
        s.function.id = self.id;
        (s, dep_plans)
    }

    fn name(&self) -> &'static str {
        self.function.type_name()
    }

    default fn execute(&self, plan: &Plan) {
        for input in &plan.inputs {
            let producer = SinkProducer::<Out>::new();
            let mut sink = producer.make_sink(&plan.outputs);
            {
                let sink_ref: &mut dyn EmitFn<Out> = &mut *sink;
                match input.format {
                    DataFormat::InMemory => {
                        let mut stream = crate::in_memory::mem_source(input);
                        while let Some(item) = stream.next() {
                            self.function.map(item, sink_ref);
                        }
                    }
                    x => panic!("I don't know how to execute format: {:?}!", x),
                }
            }
            sink.finish();
        }
    }
}
