/*
 * Library for doing dtable compaction
 */

extern crate keyserializer;
extern crate largetable_grpc_rust;
extern crate largetable_proto_rust;
extern crate sstable;

use sstable::{SSTableBuilder, SSTableReader, ShardedSSTableReader};

use keyserializer::get_colspec;
use largetable_grpc_rust::CompactionPolicy;
use largetable_proto_rust::Record;

struct Trie<T> {
    prefix: String,
    children: Vec<Trie<T>>,
    data: Option<T>,
}

impl<T> Trie<T> {
    pub fn new() -> Self {
        Self {
            prefix: String::new(),
            children: Vec::new(),
            data: None,
        }
    }

    pub fn insert_ordered(&mut self, prefix: String, data: T) {
        match self.children.last_mut() {
            Some(child) => {
                if prefix.starts_with(&child.prefix) {
                    return child.insert_ordered(prefix, data);
                }
            }
            None => (),
        };

        let mut n = Trie::new();
        n.prefix = prefix;
        n.data = Some(data);
        self.children.push(n);
    }

    pub fn lookup(&self, value: &str) -> Option<&T> {
        match self
            .children
            .binary_search_by_key(&value, |c| c.prefix.as_str())
        {
            Ok(idx) => self.children[idx].data.as_ref(),
            Err(idx) => {
                if idx == 0 {
                    return self.data.as_ref();
                }
                if value.starts_with(&self.children[idx - 1].prefix) {
                    return self.children[idx - 1].lookup(value);
                }
                self.data.as_ref()
            }
        }
    }
}

fn policy_to_key(p: &CompactionPolicy) -> String {
    get_colspec(p.get_row(), p.get_scope())
}

fn apply_policy<W: std::io::Write>(
    builder: &mut SSTableBuilder<Record, W>,
    policy: &CompactionPolicy,
    now: u64,
    buffer: Vec<(String, Record)>,
) {
    let horizon = match policy.get_ttl() {
        0 => 0,
        ttl => now - ttl,
    };
    let max_elements = match policy.get_history() {
        0 => std::usize::MAX,
        x => x as usize,
    };

    let to_write = buffer
        .into_iter()
        .rev()
        .take(max_elements)
        .filter(|(_, value)| value.get_timestamp() >= horizon)
        .collect::<Vec<_>>();

    for (key, value) in to_write.into_iter().rev() {
        builder.write_ordered(&key, value).unwrap();
    }
}

pub fn compact<W: std::io::Write>(
    mut policies: Vec<CompactionPolicy>,
    tables: Vec<SSTableReader<Record>>,
    now: u64,
    mut builder: SSTableBuilder<Record, W>,
) {
    policies.sort_by_key(|p| policy_to_key(&p));
    let mut policy_trie = Trie::new();
    for p in policies {
        policy_trie.insert_ordered(policy_to_key(&p), p);
    }

    let reader = ShardedSSTableReader::from_readers(tables, "", String::new());
    let mut buffer = Vec::new();
    let mut current_prefix = String::from("");
    let default_policy = CompactionPolicy::new();
    for (key, value) in reader {
        {
            let prefix = keyserializer::get_prefix(&key);
            if current_prefix == "" {
                current_prefix = prefix.to_owned();
            }
            if current_prefix != prefix {
                let policy = policy_trie
                    .lookup(&current_prefix)
                    .unwrap_or(&default_policy);

                apply_policy(
                    &mut builder,
                    policy,
                    now,
                    std::mem::replace(&mut buffer, Vec::new()),
                );

                current_prefix = prefix.to_owned();
                buffer.clear();
            }
        }

        buffer.push((key, value));
    }

    let policy = policy_trie
        .lookup(&current_prefix)
        .unwrap_or(&default_policy);
    apply_policy(
        &mut builder,
        policy,
        now,
        std::mem::replace(&mut buffer, Vec::new()),
    );

    builder.finish().unwrap();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Seek;

    #[test]
    fn test_trie() {
        let mut t = Trie::new();
        t.insert_ordered(String::from("a"), 0);
        t.insert_ordered(String::from("ab"), 1);
        t.insert_ordered(String::from("abc"), 2);
        t.insert_ordered(String::from("b"), 3);
        t.insert_ordered(String::from("ba"), 4);
        t.insert_ordered(String::from("bc"), 5);

        assert_eq!(t.lookup("alphabet"), Some(&0));
        assert_eq!(t.lookup("atom bomb"), Some(&0));
        assert_eq!(t.lookup("abra cadabra"), Some(&1));
        assert_eq!(t.lookup("abc onetwothree"), Some(&2));
        assert_eq!(t.lookup("abd threfourfive"), Some(&1));
        assert_eq!(t.lookup("b yourself"), Some(&3));
        assert_eq!(t.lookup("bad animal"), Some(&4));
        assert_eq!(t.lookup("bcause"), Some(&5));
        assert_eq!(t.lookup("bees"), Some(&3));
    }

    fn test_rec(data: &str) -> Record {
        let mut r = Record::new();
        r.set_data(data.to_owned().into_bytes());
        r
    }

    fn test_policy(row: &str, scope: &str, ttl: u64, history: u64) -> CompactionPolicy {
        let mut p = CompactionPolicy::new();
        p.set_row(row.to_owned());
        p.set_scope(scope.to_owned());
        p.set_ttl(ttl);
        p.set_history(history);
        p
    }

    fn write<W: std::io::Write>(
        w: &mut SSTableBuilder<Record, W>,
        row: &str,
        col: &str,
        timestamp: u64,
    ) {
        let mut r = Record::new();
        r.set_row(row.to_owned());
        r.set_col(col.to_owned());
        r.set_timestamp(timestamp);
        w.write_ordered(&keyserializer::serialize_key(row, col, timestamp), r);
    }

    #[test]
    fn test_compaction() {
        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d1);
            write(&mut t, "a", "aardvark", 100);
            write(&mut t, "a", "baseball", 100);
            write(&mut t, "a", "calendar", 100);
            write(&mut t, "a", "diamond", 100);
            t.finish().unwrap();
        }
        d1.seek(std::io::SeekFrom::Start(0)).unwrap();

        let mut d2 = std::io::Cursor::new(Vec::new());
        let bytes = d1.into_inner();
        {
            let mut r1 = SSTableReader::<Record>::from_bytes(&bytes).unwrap();
            compact(
                Vec::new(),
                vec![r1],
                150,
                SSTableBuilder::<Record, _>::new(&mut d2),
            );
        }
        d2.seek(std::io::SeekFrom::Start(0)).unwrap();

        let bytes = d2.into_inner();
        let mut s = SSTableReader::<Record>::from_bytes(&bytes).unwrap();
        let mapped = s.map(|(k, v)| v.get_col().to_owned()).collect::<Vec<_>>();
        assert_eq!(mapped, vec!["aardvark", "baseball", "calendar", "diamond"]);
    }

    #[test]
    fn test_compaction_policies() {
        let mut policies = Vec::new();
        // By default, store only one record
        policies.push(test_policy("a", "", 0, 1));
        // For all c columns, store all records
        policies.push(test_policy("a", "c", 0, 0));
        // For all b records, store only in the past 50 seconds
        policies.push(test_policy("a", "b", 50, 0));
        // For all d records, store only in the past 50 seconds
        policies.push(test_policy("a", "d", 50, 0));

        let mut d1 = std::io::Cursor::new(Vec::new());
        {
            let mut t = SSTableBuilder::<Record, _>::new(&mut d1);
            write(&mut t, "a", "aardvark", 100);
            write(&mut t, "a", "aardvark", 125);
            write(&mut t, "a", "baseball", 100);
            write(&mut t, "a", "baseball", 125);
            write(&mut t, "a", "baseball", 135);
            write(&mut t, "a", "calendar", 100);
            write(&mut t, "a", "calendar", 100);
            write(&mut t, "a", "calendar", 100);
            write(&mut t, "a", "diamond", 100);
            write(&mut t, "a", "elephant", 0);
            write(&mut t, "a", "elephant", 100);
            write(&mut t, "a", "elephant", 200);
            t.finish().unwrap();
        }
        d1.seek(std::io::SeekFrom::Start(0)).unwrap();
        let bytes = d1.into_inner();

        let mut d2 = std::io::Cursor::new(Vec::new());
        {
            let mut r1 = SSTableReader::<Record>::from_bytes(&bytes).unwrap();
            compact(
                policies,
                vec![r1],
                160,
                SSTableBuilder::<Record, _>::new(&mut d2),
            );
        }
        let bytes = d2.into_inner();
        let mut s = SSTableReader::<Record>::from_bytes(&bytes).unwrap();
        let mapped = s.map(|(k, v)| v.get_col().to_owned()).collect::<Vec<_>>();
        assert_eq!(
            mapped,
            vec![
                "aardvark", // Falls under default policy, keeps only most recent
                "baseball", "baseball", // Falls under B policy, keep only last two
                "calendar", "calendar", "calendar", // Falls under C policy, keep all
                "elephant", // falls under default policy
            ]
        );
    }
}
