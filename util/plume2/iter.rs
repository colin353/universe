pub trait StreamingIterator {
    type Item: ?Sized;

    fn advance(&mut self);
    fn get(&self) -> Option<&Self::Item>;

    fn next(&mut self) -> Option<&Self::Item> {
        self.advance();
        (*self).get()
    }
}

pub fn chain<T: 'static>(
    iters: Vec<Box<dyn StreamingIterator<Item = T>>>,
) -> Box<dyn StreamingIterator<Item = T>> {
    Box::new(JoinedStreamingIterator { iters })
}

pub struct JoinedStreamingIterator<T> {
    iters: Vec<Box<dyn StreamingIterator<Item = T>>>,
}

impl<T> StreamingIterator for JoinedStreamingIterator<T> {
    type Item = T;
    fn advance(&mut self) {
        if self.iters.is_empty() {
            return;
        }

        let len = self.iters.len();
        if self.iters[len - 1].get().is_some() {
            self.iters[len - 1].advance();
        } else {
            self.iters.pop();
        }
    }

    fn get(&self) -> Option<&Self::Item> {
        if self.iters.is_empty() {
            return None;
        }
        self.iters[self.iters.len() - 1].get()
    }
}
