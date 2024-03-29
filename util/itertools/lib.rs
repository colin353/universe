#![feature(binary_heap_into_iter_sorted)]
extern crate primitive;

use primitive::Serializable;
use std::cmp::Reverse;
use std::collections::BinaryHeap;

pub struct JoinedOrderedIterators<T, L: Iterator<Item = T>, R: Iterator<Item = T>, F> {
    left: std::iter::Peekable<L>,
    right: std::iter::Peekable<R>,
    cmp: F,
}

impl<T, L, R, F> JoinedOrderedIterators<T, L, R, F>
where
    L: Iterator<Item = T>,
    R: Iterator<Item = T>,
    F: Fn(&T, &T) -> std::cmp::Ordering,
{
    pub fn new(left: L, right: R, cmp: F) -> Self {
        Self {
            left: left.peekable(),
            right: right.peekable(),
            cmp,
        }
    }

    pub fn next(&mut self) -> (Option<T>, Option<T>) {
        match (self.left.peek(), self.right.peek()) {
            (Some(l), Some(r)) => match (self.cmp)(l, r) {
                std::cmp::Ordering::Equal => (self.left.next(), self.right.next()),
                std::cmp::Ordering::Less => (self.left.next(), None),
                std::cmp::Ordering::Greater => (None, self.right.next()),
            },
            (Some(l), None) => (self.left.next(), None),
            (None, Some(r)) => (None, self.right.next()),
            (None, None) => (None, None),
        }
    }
}

pub trait StreamingIterator {
    type Item;
    fn peek<'a>(&'a mut self) -> Option<&'a Self::Item>;
    fn next<'a>(&'a mut self) -> Option<&'a Self::Item>;
    fn count(&mut self) -> u64 {
        let mut count = 0;
        while let Some(x) = self.next() {
            count += 1;
        }
        count
    }
}

#[derive(Debug, Clone)]
pub struct KV<K, V>(pub K, pub V);

impl<K, V> KV<K, V> {
    pub fn new(k: K, v: V) -> Self {
        KV(k, v)
    }

    pub fn key(&self) -> &K {
        &self.0
    }
    pub fn value(&self) -> &V {
        &self.1
    }
}

impl<K, V> Ord for KV<K, V>
where
    K: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
    }
}

impl<K, V> PartialOrd for KV<K, V>
where
    K: Ord,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.0.cmp(&other.0))
    }
}

impl<K, V> PartialEq for KV<K, V>
where
    K: Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<K, V> Eq for KV<K, V> where K: Eq {}

impl<K, V> Serializable for KV<K, V>
where
    V: Serializable,
{
    fn write(&self, write: &mut dyn std::io::Write) -> std::io::Result<u64> {
        Ok(0)
    }
    fn read_from_bytes(&mut self, buffer: &[u8]) -> std::io::Result<()> {
        Ok(())
    }
    fn from_bytes(buffer: &[u8]) -> std::io::Result<Self> {
        unreachable!("should never directly serialize KV types!")
    }
}

impl<K, V> Default for KV<K, V>
where
    K: Default,
    V: Default,
{
    fn default() -> Self {
        KV(Default::default(), Default::default())
    }
}

#[derive(Debug)]
pub struct MinHeap<T>(BinaryHeap<Reverse<T>>);

impl<T> MinHeap<T>
where
    T: Ord,
{
    pub fn new() -> Self {
        MinHeap(BinaryHeap::new())
    }

    pub fn clear(&mut self) {
        self.0.clear()
    }

    pub fn peek(&mut self) -> Option<&T> {
        match self.0.peek() {
            Some(Reverse(out)) => Some(out),
            None => None,
        }
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn pop(&mut self) -> Option<T> {
        match self.0.pop() {
            Some(Reverse(out)) => Some(out),
            None => None,
        }
    }

    pub fn push(&mut self, v: T) {
        self.0.push(Reverse(v));
    }

    pub fn into_iter_sorted(self) -> impl Iterator<Item = T> {
        self.0.into_iter_sorted().map(|Reverse(x)| x)
    }
}
