use plume::{EmitFn, PCollection, Primitive, Stream, StreamingIterator, KV};
use search_proto_rust::*;

use std::collections::HashMap;
use std::sync::RwLock;

pub struct PageRankFn {
    decay: f32,
    ranks: RwLock<HashMap<String, f32>>,
    residual_rank: RwLock<f32>,
}
impl PageRankFn {
    pub fn new() -> Self {
        Self {
            decay: 0.15,
            ranks: RwLock::new(HashMap::new()),
            residual_rank: RwLock::new(0.0),
        }
    }
}
impl plume::DoSideInputFn for PageRankFn {
    type Input = KV<String, File>;
    type SideInput = KV<String, File>;
    type Output = KV<String, File>;

    fn init(&self, side_input: &mut dyn StreamingIterator<Item = Self::SideInput>) {
        let mut ranks = self.ranks.write().unwrap();

        // Only requires a single initialization, even if multiple shards exist
        if ranks.len() > 0 {
            return;
        }
        let mut total_rank = 0.0;
        let mut total_files = 0;
        while let Some(pair) = side_input.next() {
            let f = pair.value();

            total_files += 1;
            if f.get_imports().len() > 0 {
                let exported_rank =
                    f.get_page_rank() * (1.0 - self.decay) / (f.get_imports().len() as f32);
                ranks.insert(f.get_filename().into(), exported_rank);
            }
            total_rank += f.get_page_rank();
        }
        *(self.residual_rank.write().unwrap()) = total_rank / (total_files as f32);
    }

    fn do_it(&self, input: &KV<String, File>, emit: &mut dyn EmitFn<Self::Output>) {
        let mut f = input.value().clone();
        let ranks = self.ranks.read().unwrap();
        let residual_rank = *self.residual_rank.read().unwrap();
        let mut pagerank = residual_rank;
        for dep in f.get_dependents() {
            let contributed_rank = match ranks.get(dep) {
                Some(r) => r,
                None => continue,
            };
            pagerank += contributed_rank;
        }

        f.set_page_rank(pagerank);

        emit.emit(KV::new(input.key().clone(), f));
    }
}
