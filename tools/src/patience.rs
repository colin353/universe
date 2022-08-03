//! This crate is for computing *patience diffs*, which require more effort to compute than normal
//! diffs but are usually more human-readable.
//!
//! A diff describes the difference between two lists `a` and `b`; namely, the diff between `a` and
//! `b` describes how to go from `a` to `b` by inserting, deleting, or keeping elements.
//!
//! # Why use a patience diff?
//!
//! Patience diffs are often more readable than ordinary, [longest common
//! subsequence][wiki-lcs]-based diffs. For example, if you go from:
//!
//! ```c
//! int func_1() {
//!     return 1;
//! }
//!
//! int func_2() {
//!     return 2;
//! }
//! ```
//!
//! To:
//!
//! ```c
//! int func_1() {
//!     return 1;
//! }
//!
//! int func_new() {
//!     return 0;
//! }
//!
//! int func_2() {
//!     return 2;
//! }
//! ```
//!
//! The LCS diff between these two sequences of lines is:
//!
//! ```c
//!   int func_1() {
//!       return 1;
//! + }
//! +
//! + int func_new() {
//! +     return 0;
//!   }
//!
//!   int func_2() {
//!       return 2;
//!   }
//! ```
//!
//! Their patience diff, on the other hand, is:
//!
//! ```c
//!   int func_1() {
//!       return 1;
//!   }
//!
//! + int func_new() {
//! +     return 0;
//! + }
//! +
//!   int func_2() {
//!       return 2;
//!   }
//! ```
//!
//! ## How a patience diff is computed
//!
//! An "ordinary" diff is based on a longest common subsequence between `a` and `b`. A patience
//! diff is very similar, but first finds the longest common subsequence between the *unique*
//! elements of `a` and `b` to find "unambiguous" matches. Then, a patience diff is recursively
//! computed for ranges between matched elements.
//!
//! You can read Bram Cohen, "discoverer" of patience diff, describe patience diff in his own words
//! [here][bram-blog].
//!
//! [wiki-lcs]: https://en.wikipedia.org/wiki/Longest_common_subsequence_problem
//! [bram-blog]: http://bramcohen.livejournal.com/73318.html

extern crate lcs;

use std::collections::hash_map::{HashMap, Entry};
use std::hash::{Hash, Hasher};

#[derive(Debug, Eq)]
struct Indexed<T> {
    index: usize,
    value: T
}

impl<T> PartialEq for Indexed<T> where T: PartialEq {
    fn eq(&self, other: &Indexed<T>) -> bool {
        self.value == other.value
    }
}

impl<T> Hash for Indexed<T> where T: Hash {
    fn hash<H>(&self, state: &mut H) where H: Hasher {
        self.value.hash(state);
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiffComponent<T> {
    Insertion(T),
    Unchanged(T, T),
    Deletion(T)
}

/// Computes the patience diff betwen `a` and `b`. The `DiffComponent`s hold references to the
/// elements in `a` and `b` they correspond to.
///
/// ```
/// use patience_diff::DiffComponent;
///
/// let a: Vec<_> = "AaaxZ".chars().collect();
/// let b: Vec<_> = "AxaaZ".chars().collect();
///
/// let diff = patience_diff::patience_diff(&a, &b);
/// assert_eq!(diff, vec![
///     DiffComponent::Unchanged(&'A', &'A'),
///     DiffComponent::Deletion(&'a'),
///     DiffComponent::Deletion(&'a'),
///     DiffComponent::Unchanged(&'x', &'x'),
///     DiffComponent::Insertion(&'a'),
///     DiffComponent::Insertion(&'a'),
///     DiffComponent::Unchanged(&'Z', &'Z')
/// ]);
/// ```
pub fn patience_diff<'a, T>(a: &'a [T], b: &'a [T]) -> Vec<DiffComponent<&'a T>>
        where T: Eq + Hash {
    if a.len() == 0 && b.len() == 0 {
        return vec![];
    }

    if a.len() == 0 {
        return b.iter().map(DiffComponent::Insertion).collect();
    }

    if b.len() == 0 {
        return a.iter().map(DiffComponent::Deletion).collect();
    }

    let mut common_prefix = common_prefix(a.iter(), b.iter());
    if !common_prefix.is_empty() {
        let rest_a = &a[common_prefix.len()..];
        let rest_b = &b[common_prefix.len()..];
        common_prefix.extend(patience_diff(rest_a, rest_b));
        return common_prefix;
    }

    let common_suffix = common_suffix(a.iter(), b.iter());
    if !common_suffix.is_empty() {
        let prev_a = &a[..a.len() - common_suffix.len()];
        let prev_b = &b[..b.len() - common_suffix.len()];
        let mut prev_diff = patience_diff(prev_a, prev_b);
        prev_diff.extend(common_suffix);

        return prev_diff;
    }

    let indexed_a: Vec<_> = a.iter()
        .enumerate()
        .map(|(i, val)| Indexed { index: i, value: val })
        .collect();
    let indexed_b: Vec<_> = b.iter()
        .enumerate()
        .map(|(i, val)| Indexed { index: i, value: val })
        .collect();

    let uniq_a = unique_elements(&indexed_a);
    let uniq_b = unique_elements(&indexed_b);

    let table = lcs::LcsTable::new(&uniq_a, &uniq_b);
    let lcs = table.longest_common_subsequence();

    if lcs.is_empty() {
        let table = lcs::LcsTable::new(&indexed_a, &indexed_b);
        return table.diff().into_iter().map(|c| {
            match c {
                lcs::DiffComponent::Insertion(elem_b) => {
                    DiffComponent::Insertion(&b[elem_b.index])
                },
                lcs::DiffComponent::Unchanged(elem_a, elem_b) => {
                    DiffComponent::Unchanged(&a[elem_a.index], &b[elem_b.index])
                },
                lcs::DiffComponent::Deletion(elem_a) => {
                    DiffComponent::Deletion(&a[elem_a.index])
                }
            }
        }).collect();
    }

    let mut ret = Vec::new();
    let mut last_index_a = 0;
    let mut last_index_b = 0;

    for (match_a, match_b) in lcs {
        let subset_a = &a[last_index_a..match_a.index];
        let subset_b = &b[last_index_b..match_b.index];

        ret.extend(patience_diff(subset_a, subset_b));

        ret.push(DiffComponent::Unchanged(match_a.value, match_b.value));

        last_index_a = match_a.index + 1;
        last_index_b = match_b.index + 1;
    }

    let subset_a = &a[last_index_a..a.len()];
    let subset_b = &b[last_index_b..b.len()];
    ret.extend(patience_diff(subset_a, subset_b));

    ret
}

fn common_prefix<'a, T, I>(a: I, b: I) -> Vec<DiffComponent<I::Item>>
        where I: Iterator<Item = &'a T>, T: Eq {
    a.zip(b)
        .take_while(|&(elem_a, elem_b)| elem_a == elem_b)
        .map(|(elem_a, elem_b)| DiffComponent::Unchanged(elem_a, elem_b))
        .collect()
}

fn common_suffix<'a, T, I>(a: I, b: I) -> Vec<DiffComponent<I::Item>>
        where I: DoubleEndedIterator<Item = &'a T>, T: Eq {
    common_prefix(a.rev(), b.rev())
}

fn unique_elements<'a, T: Eq + Hash>(elems: &'a [T]) -> Vec<&'a T> {
    let mut counts: HashMap<&T, usize> = HashMap::new();

    for elem in elems {
        match counts.entry(elem) {
            Entry::Occupied(mut e) => {
                *e.get_mut() = e.get() + 1;
            },
            Entry::Vacant(e) => {
                e.insert(1);
            }
        }
    }

    elems.iter()
        .filter(|elem| counts.get(elem) == Some(&1))
        .collect()
}

#[test]
fn test_patience_diff() {
    let a: Vec<_> = "aax".chars().collect();
    let b: Vec<_> = "xaa".chars().collect();

    let diff = patience_diff(&a, &b);
    assert_eq!(diff, vec![
        DiffComponent::Deletion(&'a'),
        DiffComponent::Deletion(&'a'),
        DiffComponent::Unchanged(&'x', &'x'),
        DiffComponent::Insertion(&'a'),
        DiffComponent::Insertion(&'a'),
    ]);

    let a = vec![1, 10, 11, 4];
    let b = vec![1, 10, 11, 2, 3, 10, 11, 4];

    let diff = patience_diff(&a, &b);
    assert_eq!(diff, vec![
        DiffComponent::Unchanged(&1, &1),
        DiffComponent::Unchanged(&10, &10),
        DiffComponent::Unchanged(&11, &11),
        DiffComponent::Insertion(&2),
        DiffComponent::Insertion(&3),
        DiffComponent::Insertion(&10),
        DiffComponent::Insertion(&11),
        DiffComponent::Unchanged(&4, &4)
    ]);
}

#[test]
fn test_unique_elements() {
    assert_eq!(vec![&2, &4, &5], unique_elements(&[1, 2, 3, 3, 4, 5, 1]));
}
