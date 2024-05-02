use plume::{Collection, EmitFn, MapFn};

struct Mapper {}
impl MapFn for Mapper {
    type Input = u64;
    type Output = u64;
    fn map(&self, input: &u64, emit: &mut dyn EmitFn<Self::Output>) {
        emit.emit(*input * 2);
    }
}

fn main() {
    let p = Collection::from_vec(vec![1, 2, 3, 4]);
    let out = p.map(Mapper {});
    out.write_to_vec();

    println!("hello world");
    plume::inspect();

    plume::run();

    println!("output: {:#?}", out.as_vec());
}
