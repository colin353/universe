/*
 * compaction.rs
 *
 * Library for doing dtable compaction
 */

extern crate keyserializer;
extern crate largetable_proto_rust;
extern crate sstable;

use keyserializer::get_colspec;
use largetable_proto_rust::{CompactionPolicy, Record};

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

    pub fn render(&self) -> String {
        if self.children.is_empty() {
            return format!("`{}`", self.prefix);
        }
        return format!(
            "`{}` => ( {} )",
            self.prefix,
            self.children
                .iter()
                .map(|c| c.render())
                .collect::<Vec<_>>()
                .join(", ")
        );
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

fn get_timestamp_usec() -> u64 {
    let tm = time::now_utc().to_timespec();
    (tm.sec as u64) * 1_000_000 + ((tm.nsec / 1000) as u64)
}

fn apply_policy(
    builder: &mut sstable::SSTableBuilder<Record>,
    policy: &CompactionPolicy,
    buffer: Vec<(String, Record)>,
) {
    let horizon = match policy.get_ttl() {
        0 => 0,
        ttl => get_timestamp_usec() - ttl,
    };
    let max_elements = match policy.get_history() {
        0 => std::usize::MAX,
        x => x as usize,
    };

    for (key, value) in buffer
        .into_iter()
        .rev()
        .take(max_elements)
        .filter(|(_, value)| value.get_timestamp() < horizon)
    {
        builder.write_ordered(&key, value);
    }
}

fn compact(
    mut policies: Vec<CompactionPolicy>,
    tables: Vec<sstable::SSTableReader<Record>>,
    builder: &mut sstable::SSTableBuilder<Record>,
) {
    policies.sort_by_key(|p| policy_to_key(&p));
    let mut policy_trie = Trie::new();
    for p in policies {
        policy_trie.insert_ordered(policy_to_key(&p), p);
    }

    let reader = sstable::ShardedSSTableReader::from_readers(tables, "", String::new());
    let mut buffer = Vec::new();
    let mut current_prefix = String::from("");
    let default_policy = CompactionPolicy::new();
    for (key, value) in reader {
        {
            let prefix = keyserializer::get_prefix(&key);
            if current_prefix == "" {
                current_prefix = prefix.to_owned();
            }
            if current_prefix != prefix.to_owned() {
                let policy = policy_trie
                    .lookup(&current_prefix)
                    .unwrap_or(&default_policy);
                apply_policy(builder, policy, std::mem::replace(&mut buffer, Vec::new()));

                current_prefix = prefix.to_owned();
                buffer.clear();
            }
        }

        buffer.push((key, value));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_compaction() {
        // idk what to do here
    }
}
