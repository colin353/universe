extern crate plume;
use plume::EmitFn;
use plume::PCollection;

struct MyDoFn {}
impl plume::DoFn for MyDoFn {
    type Input = u64;
    type Output = f64;
    fn do_it(&self, input: &u64, emit: &mut dyn EmitFn<f64>) {
        emit.emit(*input as f64 / 32.0);
    }
}

struct MyMapperFn {}
impl plume::DoFn for MyMapperFn {
    type Input = f64;
    type Output = String;
    fn do_it(&self, input: &f64, emit: &mut dyn EmitFn<String>) {
        emit.emit(format!("{}", *input));
    }
}

fn main() {
    let p = PCollection::<u64>::new();
    let o = p.par_do(MyDoFn {});
    let o = o.par_do(MyMapperFn {});
    plume::compute(o);
}
