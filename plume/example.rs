extern crate plume;
use plume::EmitFn;
use plume::PCollection;
use plume::PTable;
use plume::Stream;

struct Do1 {}
impl plume::DoFn for Do1 {
    type Input = u64;
    type Output = (String, f64);
    fn do_it(&self, input: u64, emit: &mut dyn EmitFn<(String, f64)>) {
        emit.emit((String::from("k"), input as f64 / 32.0));
    }
}

struct Do2 {}
impl plume::DoFn for Do2 {
    type Input = u64;
    type Output = (String, u8);
    fn do_it(&self, input: u64, emit: &mut dyn EmitFn<(String, u8)>) {
        emit.emit((String::from("k"), 6));
    }
}

struct MyJoinFn {}
impl plume::JoinFn for MyJoinFn {
    type Key = String;
    type ValueLeft = f64;
    type ValueRight = u8;
    type Output = (String, String);
    fn join(
        &self,
        key: String,
        left: Stream<f64>,
        right: Stream<u8>,
        emit: &mut dyn EmitFn<(String, String)>,
    ) {
        emit.emit((String::from("1"), String::from("aaa")));
    }
}

fn main() {
    let p = PCollection::<u64>::from_vec(vec![1, 2, 3, 4]);
    let o1 = p.par_do(Do1 {});
    let o2 = p.par_do(Do2 {});
    let joined = o1.join(o2, MyJoinFn {});
    let output = joined.group_by_key();

    plume::run(output);
}
