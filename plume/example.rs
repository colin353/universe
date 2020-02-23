extern crate plume;
use plume::EmitFn;
use plume::PCollection;
use plume::PTable;
use plume::Stream;
use plume::KV;

struct Do1 {}
impl plume::DoFn for Do1 {
    type Input = u64;
    type Output = KV<String, u8>;
    fn do_it(&self, input: &u64, emit: &mut dyn EmitFn<Self::Output>) {
        println!("DoFn: got {:?}", input);
        emit.emit(KV::new(format!("{:?}", (*input)), *input as u8));
    }
}

struct Do2 {}
impl plume::DoFn for Do2 {
    type Input = KV<String, Stream<u8>>;
    type Output = KV<String, u32>;
    fn do_it(&self, input: &KV<String, Stream<u8>>, emit: &mut dyn EmitFn<Self::Output>) {
        println!("DoFn2: got a real stream");
        emit.emit(KV::new(input.key().clone(), 5));
    }
}

struct MyJoinFn {}
impl plume::JoinFn for MyJoinFn {
    type Key = String;
    type ValueLeft = f64;
    type ValueRight = u8;
    type Output = KV<String, String>;
    fn join(
        &self,
        key: String,
        left: Stream<f64>,
        right: Stream<u8>,
        emit: &mut dyn EmitFn<KV<String, String>>,
    ) {
        emit.emit(KV::new(String::from("1"), String::from("aaa")));
    }
}

fn main() {
    let p = PCollection::<u64>::from_vec(vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]);
    let o1 = p.par_do(Do1 {}).group_by_key();
    let mut o2 = o1.par_do(Do2 {});
    //let o2 = p.par_do(Do2 {});
    //let joined = o1.join(o2, MyJoinFn {});
    //let output = joined.group_by_key();
    //o2.write_to_sstabe("/home/colin/output.sstable@2");
    o2.write_to_vec();

    //let t = PCollection::<(String, u64)>::from_table(vec![("A".into(), 1), ("B".into(), 1)]);

    plume::run();

    println!("result: {:?}", o2.into_vec());
}
