extern crate plume;
use plume::EmitFn;
use plume::PCollection;
use plume::Stream;
use plume::KV;

struct Do1 {}
impl plume::DoFn for Do1 {
    type Input = u64;
    type Output = KV<String, u8>;
    fn do_it(&self, input: &u64, emit: &mut dyn EmitFn<Self::Output>) {
        println!("DoFn: got {:?}", input);
        emit.emit(KV::new(format!("{:?}", *input), 1));
    }
}

struct Do2 {}
impl plume::DoStreamFn for Do2 {
    type Input = u8;
    type Output = KV<String, u32>;
    fn do_it(&self, key: &str, values: &mut Stream<u8>, emit: &mut dyn EmitFn<Self::Output>) {
        let mut sum: u32 = 0;
        for value in values {
            sum += *value as u32;
        }
        println!("grouped: {} --> {}", key, sum);
        emit.emit(KV::new(key.to_string(), sum));
    }
}

struct Do3 {}
impl plume::DoFn for Do3 {
    type Input = KV<String, u32>;
    type Output = KV<String, u32>;
    fn do_it(&self, input: &KV<String, u32>, emit: &mut dyn EmitFn<Self::Output>) {
        println!("identity map: {}", input.key());
        emit.emit(KV::new(input.key().to_string(), 0));
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
    let p = PCollection::<u64>::from_vec(vec![1, 1, 2, 3, 4, 5, 6, 7, 1, 8, 9, 10, 11, 1, 1]);
    let o1 = p.par_do(Do1 {});
    let o2 = o1.group_by_key_and_par_do(Do2 {});
    let mut o3 = o2.par_do(Do3 {});
    //let o2 = p.par_do(Do2 {});
    //let joined = o1.join(o2, MyJoinFn {});
    //let output = joined.group_by_key();
    //o2.write_to_sstabe("/home/colin/output.sstable@2");
    o3.write_to_vec();

    //let t = PCollection::<(String, u64)>::from_table(vec![("A".into(), 1), ("B".into(), 1)]);

    plume::run();

    println!("result: {:?}", o3.into_vec());
}
