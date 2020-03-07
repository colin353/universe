extern crate plume;
use plume::EmitFn;
use plume::PCollection;
use plume::PTable;
use plume::Primitive;
use plume::Stream;
use plume::StreamingIterator;
use plume::KV;

struct Do1 {}
impl plume::DoFn for Do1 {
    type Input = Primitive<u64>;
    type Output = KV<String, Primitive<u64>>;
    fn do_it(&self, input: &Primitive<u64>, emit: &mut dyn EmitFn<Self::Output>) {
        println!("DoFn: got {:?}", input);
        emit.emit(KV::new(format!("{:04}", **input), 1.into()));
    }
}

struct Do2 {}
impl plume::DoStreamFn for Do2 {
    type Input = Primitive<u64>;
    type Output = KV<String, Primitive<u64>>;
    fn do_it(
        &self,
        key: &str,
        values: &mut Stream<Primitive<u64>>,
        emit: &mut dyn EmitFn<Self::Output>,
    ) {
        let mut sum: u64 = 0;
        while let Some(value) = values.next() {
            sum += (**value) as u64;
        }
        println!("grouped: {} --> {}", key, sum);
        emit.emit(KV::new(key.to_string(), sum.into()));
    }
}

struct Do3 {}
impl plume::DoFn for Do3 {
    type Input = KV<String, Primitive<u64>>;
    type Output = KV<String, Primitive<u64>>;
    fn do_it(&self, input: &KV<String, Primitive<u64>>, emit: &mut dyn EmitFn<Self::Output>) {
        println!("identity map: {} --> {}", input.key(), input.value());
        emit.emit(KV::new(input.key().to_string(), input.value().clone()));
    }
}

struct MyJoinFn {}
impl plume::JoinFn for MyJoinFn {
    type ValueLeft = Primitive<f64>;
    type ValueRight = Primitive<u64>;
    type Output = KV<String, Primitive<String>>;
    fn join(
        &self,
        key: &str,
        left: &mut Stream<Primitive<f64>>,
        right: &mut Stream<Primitive<u64>>,
        emit: &mut dyn EmitFn<KV<String, Primitive<String>>>,
    ) {
        emit.emit(KV::new(String::from("1"), String::from("aaa").into()));
    }
}

fn main() {
    /*let p = PTable::<String, Primitive<u64>>::from_sstable("/tmp/output.sstable@2");
    let mut o = p.par_do(Do3 {});
    o.write_to_vec();

    plume::run();

    return;*/

    let p = PCollection::<Primitive<u64>>::from_primitive_vec(vec![
        1, 1, 2, 3, 4, 5, 6, 7, 1, 8, 9, 10, 11, 1, 1,
    ]);
    let o1 = p.par_do(Do1 {});
    let o2 = o1.group_by_key_and_par_do(Do2 {});
    let mut o3 = o2.par_do(Do3 {});
    o3.write_to_sstable("/tmp/output.sstable@2");

    plume::run();
}
