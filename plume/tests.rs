extern crate plume;
extern crate recordio;
extern crate sstable;

use plume::EmitFn;
use plume::Stream;
use plume::StreamingIterator;
use plume::KV;
use plume::{PCollection, PCollectionProto, PTable, Primitive};

use std::collections::HashSet;
use std::sync::RwLock;

struct MapSquareFn {}
impl plume::DoFn for MapSquareFn {
    type Input = Primitive<u64>;
    type Output = Primitive<u64>;
    fn do_it(&self, input: &Primitive<u64>, emit: &mut dyn EmitFn<Self::Output>) {
        emit.emit(((**input) * (**input)).into());
    }
}

struct MapEvenOddFn {}
impl plume::DoFn for MapEvenOddFn {
    type Input = Primitive<u64>;
    type Output = KV<String, Primitive<u64>>;
    fn do_it(&self, input: &Primitive<u64>, emit: &mut dyn EmitFn<Self::Output>) {
        let key = match (**input % 2) == 0 {
            true => String::from("even"),
            false => String::from("odd"),
        };
        emit.emit(KV::new(key, (*input).into()));
    }
}

struct GroupSumFn {}
impl plume::DoStreamFn for GroupSumFn {
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
            sum += **value;
        }
        emit.emit(KV::new(key.to_string(), sum.into()));
    }
}

struct EmpJoinFn {}
impl plume::JoinFn for EmpJoinFn {
    type ValueLeft = Primitive<String>;
    type ValueRight = Primitive<String>;
    type Output = Primitive<String>;
    fn join(
        &self,
        key: &str,
        left: &mut Stream<Primitive<String>>,
        right: &mut Stream<Primitive<String>>,
        emit: &mut dyn EmitFn<Self::Output>,
    ) {
        let job = match left.next() {
            Some(x) => x,
            None => return,
        };
        let name = match right.next() {
            Some(x) => x,
            None => return,
        };
        emit.emit(format!("{}, who is a {}", name, job).into());
    }
}

struct DoNothingFn {}
impl plume::DoFn for DoNothingFn {
    type Input = KV<String, Primitive<String>>;
    type Output = KV<String, Primitive<String>>;
    fn do_it(&self, input: &Self::Input, emit: &mut dyn EmitFn<Self::Output>) {
        emit.emit(KV::new(input.key().to_owned(), input.value().to_owned()));
    }
}

struct DoSideInputTestFn {
    side_map: RwLock<HashSet<String>>,
}
impl DoSideInputTestFn {
    fn new() -> Self {
        Self {
            side_map: RwLock::new(HashSet::new()),
        }
    }
}

impl plume::DoSideInputFn for DoSideInputTestFn {
    type Input = Primitive<String>;
    type SideInput = Primitive<String>;
    type Output = Primitive<String>;

    fn init(&self, input: &mut dyn StreamingIterator<Item = Self::SideInput>) {
        let mut side_map = self.side_map.write().unwrap();
        while let Some(x) = input.next() {
            side_map.insert(x.to_string());
        }
    }

    fn do_it(&self, input: &Self::Input, emit: &mut dyn EmitFn<Self::Output>) {
        if self.side_map.read().unwrap().contains(input.as_str()) {
            emit.emit(input.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let p = PCollection::from_primitive_vec(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let mut out = p.par_do(MapSquareFn {});
        out.write_to_vec();
        plume::run();
        assert_eq!(
            out.into_vec().iter().map(|x| *x).collect::<Vec<_>>(),
            vec![1, 4, 9, 16, 25, 36, 49, 64]
        );
    }

    #[test]
    fn test_multiple_mapping() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let p = PCollection::from_primitive_vec(vec![1, 2, 3, 4]);
        let squared = p.par_do(MapSquareFn {});
        let mut evenodd = squared.par_do(MapEvenOddFn {});
        evenodd.write_to_vec();
        plume::run();
        assert_eq!(
            evenodd.into_vec().as_ref(),
            &vec![
                KV::new(String::from("even"), 4.into()),
                KV::new(String::from("even"), 16.into()),
                KV::new(String::from("odd"), 1.into()),
                KV::new(String::from("odd"), 9.into()),
            ]
        );
    }

    #[test]
    fn test_multiple_mapping_with_group_by() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let p = PCollection::from_primitive_vec(vec![1, 2, 3, 4]);
        let squared = p.par_do(MapSquareFn {});
        let evenodd = squared.par_do(MapEvenOddFn {});
        let mut grouped = evenodd.group_by_key_and_par_do(GroupSumFn {});
        grouped.write_to_vec();
        plume::run();
        assert_eq!(
            grouped.into_vec().as_ref(),
            &vec![
                KV::new(String::from("even"), 20.into()),
                KV::new(String::from("odd"), 10.into()),
            ]
        );
    }

    #[test]
    fn test_joining() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let emp_types = PTable::<String, Primitive<String>>::from_table(vec![
            KV::new(String::from("1"), String::from("janitor").into()),
            KV::new(String::from("2"), String::from("sales").into()),
            KV::new(String::from("3"), String::from("marketing").into()),
            KV::new(String::from("4"), String::from("marketing").into()),
        ]);
        let emp_names = PTable::<String, Primitive<String>>::from_table(vec![
            KV::new(String::from("1"), String::from("john").into()),
            KV::new(String::from("3"), String::from("tim").into()),
            KV::new(String::from("5"), String::from("james").into()),
        ]);
        let mut joined = emp_types.join(emp_names, EmpJoinFn {});
        joined.write_to_vec();

        plume::run();
        assert_eq!(
            joined.into_vec().as_ref(),
            &vec![
                String::from("john, who is a janitor"),
                String::from("tim, who is a marketing"),
            ]
        );
    }

    #[test]
    fn test_side_input() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let emp_types = PCollection::<Primitive<String>>::from_primitive_vec(vec![
            String::from("tim").into(),
            String::from("john").into(),
            String::from("colin").into(),
            String::from("william").into(),
        ]);

        let emp_names = PCollection::<Primitive<String>>::from_primitive_vec(vec![
            String::from("tim").into(),
            String::from("colin").into(),
            String::from("josh").into(),
            String::from("kyle").into(),
        ]);
        let mut output = emp_types.par_do_side_input(DoSideInputTestFn::new(), emp_names);
        output.write_to_vec();

        plume::run();
        assert_eq!(
            output.into_vec().as_ref(),
            &vec![String::from("tim"), String::from("colin"),]
        );
    }

    // This is a very slow test which writes to disk, so let's not turn it on
    // by default
    // #[test]
    fn test_write_to_disk() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let RECORD_COUNT = 1_000_000;

        std::fs::remove_dir_all("/tmp/test-write-to-disk");
        std::fs::create_dir_all("/tmp/test-write-to-disk").unwrap();

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/input.sstable").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = sstable::SSTableBuilder::new(&mut writer);

            for idx in 0..RECORD_COUNT {
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(String::from("lorem ipsum dolor sit amet, neque porro quisquam est qui dolorem ipsum quia dolor sit amet, consectetur, adipisci velit")));
            }
            builder.finish().unwrap();
        }

        let p = PTable::<String, Primitive<String>>::from_sstable(
            "/tmp/test-write-to-disk/input.sstable",
        );
        let out = p.par_do(DoNothingFn {});
        out.write_to_sstable("/tmp/test-write-to-disk/output.sstable@2");

        plume::run();
        plume::cleanup();

        let p = PTable::<String, Primitive<String>>::from_sstable(
            "/tmp/test-write-to-disk/output.sstable@2",
        );
        let mut out = p.par_do(DoNothingFn {});
        out.write_to_vec();
        plume::run();

        let result = out.into_vec();
        assert_eq!(result.len(), RECORD_COUNT);

        let mut count = 0;
        for element in result.as_ref() {
            assert_eq!(element.key(), &format!("{:06}", count));
            count += 1;
        }
    }

    // This is a very slow test which writes to disk, so let's not turn it on
    // by default
    //#[test]
    fn test_group_from_sstable() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let RECORD_COUNT = 100;

        std::fs::remove_dir_all("/tmp/test-write-to-disk");
        std::fs::create_dir_all("/tmp/test-write-to-disk").unwrap();

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/input.sstable").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = sstable::SSTableBuilder::new(&mut writer);

            for idx in 0..RECORD_COUNT {
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(12 as u64));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(23));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(34));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(45));
            }
            builder.finish().unwrap();
        }

        let p =
            PTable::<String, Primitive<u64>>::from_sstable("/tmp/test-write-to-disk/input.sstable");
        let mut out = p.group_by_key_and_par_do(GroupSumFn {});
        out.write_to_vec();
        plume::run();

        let result = out.into_vec();
        assert_eq!(result.len(), RECORD_COUNT);

        let mut count = 0;
        for element in result.as_ref() {
            assert_eq!(element.key(), &format!("{:06}", count));
            assert_eq!(element.value(), &114);
            count += 1;
        }
    }

    // This is a very slow test which writes to disk, so let's not turn it on
    // by default
    //#[test]
    fn test_join_sstable() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        std::fs::remove_dir_all("/tmp/test-write-to-disk");
        std::fs::create_dir_all("/tmp/test-write-to-disk").unwrap();

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/names.sstable").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = sstable::SSTableBuilder::new(&mut writer);

            builder.write_ordered("1", Primitive::from(String::from("Tim")));
            builder.write_ordered("2", Primitive::from(String::from("Colin")));
            builder.write_ordered("3", Primitive::from(String::from("John")));
            builder.write_ordered("4", Primitive::from(String::from("Paul")));
            builder.finish().unwrap();
        }

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/jobs.sstable").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = sstable::SSTableBuilder::new(&mut writer);

            builder.write_ordered("2", Primitive::from(String::from("barber")));
            builder.write_ordered("4", Primitive::from(String::from("ghost")));
            builder.write_ordered("6", Primitive::from(String::from("artist")));
            builder.write_ordered("8", Primitive::from(String::from("author")));
            builder.finish().unwrap();
        }

        let names = PTable::<String, Primitive<String>>::from_sstable(
            "/tmp/test-write-to-disk/names.sstable",
        );
        let jobs = PTable::<String, Primitive<String>>::from_sstable(
            "/tmp/test-write-to-disk/jobs.sstable",
        );
        let mut out = jobs.join(names, EmpJoinFn {});
        out.write_to_vec();
        plume::run();

        let result = out.into_vec();
        assert_eq!(result.len(), 2);

        let mut iter = result.as_ref().iter();

        assert_eq!(
            *(iter.next().unwrap()),
            "Colin, who is a barber".to_string()
        );
        assert_eq!(*(iter.next().unwrap()), "Paul, who is a ghost".to_string());
    }

    // This is a very slow test which writes to disk, so let's not turn it on
    // by default
    //#[test]
    fn test_multiple_parallel_ops() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let RECORD_COUNT = 100;

        std::fs::remove_dir_all("/tmp/test-write-to-disk");
        std::fs::create_dir_all("/tmp/test-write-to-disk").unwrap();

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/input.sstable").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = sstable::SSTableBuilder::new(&mut writer);

            for idx in 0..RECORD_COUNT {
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(12 as u64));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(23));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(34));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(45));
            }
            builder.finish().unwrap();
        }

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/input2.sstable").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = sstable::SSTableBuilder::new(&mut writer);

            for idx in 0..RECORD_COUNT {
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(12 as u64));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(23));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(34));
                builder.write_ordered(&format!("{:06}", idx), Primitive::from(45));
            }
            builder.finish().unwrap();
        }

        let p =
            PTable::<String, Primitive<u64>>::from_sstable("/tmp/test-write-to-disk/input.sstable");
        let mut out = p.group_by_key_and_par_do(GroupSumFn {});
        out.write_to_sstable("/tmp/test-write-to-disk/output.sstable");

        let p2 = PTable::<String, Primitive<u64>>::from_sstable(
            "/tmp/test-write-to-disk/input2.sstable",
        );
        let mut out2 = p.group_by_key_and_par_do(GroupSumFn {});
        out2.write_to_sstable("/tmp/test-write-to-disk/output2.sstable");

        plume::run();

        assert!(std::path::Path::new("/tmp/test-write-to-disk/output.sstable").exists());
        assert!(std::path::Path::new("/tmp/test-write-to-disk/output2.sstable").exists());
    }

    #[test]
    fn test_recordio_reading() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        std::fs::remove_dir_all("/tmp/test-write-to-disk");
        std::fs::create_dir_all("/tmp/test-write-to-disk").unwrap();

        {
            let f = std::fs::File::create("/tmp/test-write-to-disk/input.recordio").unwrap();
            let mut writer = std::io::BufWriter::new(f);
            let mut builder = recordio::RecordIOWriter::new(&mut writer);

            builder.write(&Primitive::from(1 as u64));
            builder.write(&Primitive::from(2 as u64));
            builder.write(&Primitive::from(3 as u64));
            builder.write(&Primitive::from(4 as u64));
            builder.write(&Primitive::from(5 as u64));
        }

        let p = PCollection::from_recordio("/tmp/test-write-to-disk/input.recordio");
        let mut out = p.par_do(MapSquareFn {});
        out.write_to_vec();
        plume::run();
        assert_eq!(
            out.into_vec().iter().map(|x| **x).collect::<Vec<_>>(),
            vec![1, 4, 9, 16, 25]
        );
    }
}
