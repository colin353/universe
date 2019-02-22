use std::cmp::Eq;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::RwLock;

pub struct Cache<K, V> {
    max_size: usize,
    map: RwLock<HashMap<K, V>>,
}

impl<K: Eq + Hash + Clone, V: Clone> Cache<K, V> {
    pub fn new(max_size: usize) -> Self {
        Cache {
            max_size: max_size,
            map: RwLock::new(HashMap::with_capacity(max_size)),
        }
    }

    pub fn get(&self, key: &K) -> Option<V> {
        self.map.read().unwrap().get(key).cloned()
    }

    pub fn insert(&self, key: K, value: V) {
        let mut writable_map = self.map.write().unwrap();

        // If the map has already reached the max size, and we are inserting a new element, need to
        // drop a key.
        if writable_map.len() == self.max_size && writable_map.get(&key).is_none() {
            let k = writable_map.keys().next().unwrap().clone();
            writable_map.remove(&k);
        }

        writable_map.insert(key, value);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_map() {
        let c = Cache::new(5);
        c.insert(5, 10);
        c.insert(6, 12);
        c.insert(7, 14);

        assert_eq!(c.get(&5), Some(10));
    }

    #[test]
    fn test_map_2() {
        let c = Cache::new(5);
        c.insert(&1, 10);
        c.insert(&2, 12);
        c.insert(&3, 14);
        c.insert(&4, 10);
        c.insert(&5, 12);
        c.insert(&6, 10);
        c.insert(&7, 14);
        c.insert(&8, 10);
        c.insert(&9, 12);

        assert_eq!(c.map.read().unwrap().len(), 5);
    }
}
