extern crate plume;
use plume::EmitFn;
use plume::PCollection;
use plume::Stream;
use plume::KV;

struct MapSquareFn {}
impl plume::DoFn for MapSquareFn {
    type Input = u64;
    type Output = u64;
    fn do_it(&self, input: &u64, emit: &mut dyn EmitFn<Self::Output>) {
        emit.emit((*input) * (*input));
    }
}

struct MapEvenOddFn {}
impl plume::DoFn for MapEvenOddFn {
    type Input = u64;
    type Output = KV<String, u64>;
    fn do_it(&self, input: &u64, emit: &mut dyn EmitFn<Self::Output>) {
        let key = match (*input % 2) == 0 {
            true => String::from("even"),
            false => String::from("odd"),
        };
        emit.emit(KV::new(key, *input));
    }
}

struct GroupSumFn {}
impl plume::DoStreamFn for GroupSumFn {
    type Input = u64;
    type Output = KV<String, u64>;
    fn do_it(&self, key: &str, values: &mut Stream<u64>, emit: &mut dyn EmitFn<Self::Output>) {
        let mut sum: u64 = 0;
        for value in values {
            sum += *value;
        }
        emit.emit(KV::new(key.to_string(), sum));
    }
}

struct EmpJoinFn {}
impl plume::JoinFn for EmpJoinFn {
    type ValueLeft = String;
    type ValueRight = String;
    type Output = String;
    fn join(
        &self,
        key: &str,
        left: &mut Stream<String>,
        right: &mut Stream<String>,
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
        emit.emit(format!("{}, who is a {}", name, job));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mapping() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let p = PCollection::from_vec(vec![1, 2, 3, 4, 5, 6, 7, 8]);
        let mut out = p.par_do(MapSquareFn {});
        out.write_to_vec();
        plume::run();
        assert_eq!(out.into_vec().as_ref(), &vec![1, 4, 9, 16, 25, 36, 49, 64]);
    }

    #[test]
    fn test_multiple_mapping() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let p = PCollection::from_vec(vec![1, 2, 3, 4]);
        let squared = p.par_do(MapSquareFn {});
        let mut evenodd = squared.par_do(MapEvenOddFn {});
        evenodd.write_to_vec();
        plume::run();
        assert_eq!(
            evenodd.into_vec().as_ref(),
            &vec![
                KV::new(String::from("even"), 4),
                KV::new(String::from("even"), 16),
                KV::new(String::from("odd"), 1),
                KV::new(String::from("odd"), 9),
            ]
        );
    }

    #[test]
    fn test_multiple_mapping_with_group_by() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let p = PCollection::from_vec(vec![1, 2, 3, 4]);
        let squared = p.par_do(MapSquareFn {});
        let evenodd = squared.par_do(MapEvenOddFn {});
        let mut grouped = evenodd.group_by_key_and_par_do(GroupSumFn {});
        grouped.write_to_vec();
        plume::run();
        assert_eq!(
            grouped.into_vec().as_ref(),
            &vec![
                KV::new(String::from("even"), 20),
                KV::new(String::from("odd"), 10),
            ]
        );
    }

    #[test]
    fn test_joining() {
        let mut _runlock = plume::RUNLOCK.lock();
        plume::cleanup();

        let emp_types = PCollection::from_table(vec![
            KV::new(String::from("1"), String::from("janitor")),
            KV::new(String::from("2"), String::from("sales")),
            KV::new(String::from("3"), String::from("marketing")),
            KV::new(String::from("4"), String::from("marketing")),
        ]);
        let emp_names = PCollection::from_table(vec![
            KV::new(String::from("1"), String::from("john")),
            KV::new(String::from("3"), String::from("tim")),
            KV::new(String::from("5"), String::from("james")),
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
}
