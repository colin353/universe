extern crate plume;
use plume::EmitFn;
use plume::PCollection;
use plume::PTable;
use plume::Stream;

struct MyDoFn {}
impl plume::DoFn for MyDoFn {
    type Input = u64;
    type Output = f64;
    fn do_it(&self, input: u64, emit: &mut dyn EmitFn<f64>) {
        emit.emit(input as f64 / 32.0);
    }
}

struct MyMapperFn {}
impl plume::DoFn for MyMapperFn {
    type Input = f64;
    type Output = (String, f64);
    fn do_it(&self, input: f64, emit: &mut dyn EmitFn<(String, f64)>) {
        emit.emit((format!("{}", input), input));
    }
}

struct AnotherMapperFn {}
impl plume::DoFn for AnotherMapperFn {
    type Input = u128;
    type Output = (String, bool);
    fn do_it(&self, input: u128, emit: &mut dyn EmitFn<(String, bool)>) {
        emit.emit((format!("{}", input), true));
    }
}

struct MyJoinFn {}
impl plume::JoinFn for MyJoinFn {
    type Key = String;
    type ValueLeft = f64;
    type ValueRight = bool;
    type Output = u8;
    fn join(&self, key: String, left: Stream<f64>, right: Stream<bool>, emit: &mut dyn EmitFn<u8>) {
        emit.emit(1);
    }
}

fn main() {
    let p = PCollection::<u64>::new();
    let o = p.par_do(MyDoFn {});
    let o = o.par_do(MyMapperFn {});
    let x = PCollection::<u128>::new();
    let j = x.par_do(AnotherMapperFn {});
    let joined = o.join(j, MyJoinFn {});
    plume::compute(joined);
}
