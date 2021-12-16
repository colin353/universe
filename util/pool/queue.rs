use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};

#[derive(Clone)]
pub struct PoolQueue<T> {
    inner: Arc<PoolQueueInner<T>>,
}

pub struct PoolQueueInner<T> {
    queue: Mutex<VecDeque<T>>,
    alarm: Condvar,
    done: Condvar,
    waiting: Mutex<usize>,
    thread_count: usize,
}

impl<T: Send + 'static> PoolQueue<T> {
    pub fn new(size: usize) -> Self {
        let inner = Arc::new(PoolQueueInner {
            queue: Mutex::new(VecDeque::new()),
            alarm: Condvar::new(),
            done: Condvar::new(),
            waiting: Mutex::new(0),
            thread_count: size,
        });

        Self { inner }
    }

    pub fn start<F: Fn(T) + Clone + Send + Sync + 'static>(&self, operation: F) {
        for _ in 0..self.inner.thread_count {
            let _inner = self.inner.clone();
            let _op = operation.clone();
            std::thread::spawn(move || loop {
                let task = { _inner.queue.lock().unwrap().pop_front() };

                if let Some(t) = task {
                    _op(t);
                    continue;
                }

                {
                    *_inner.waiting.lock().unwrap() += 1;
                }

                _inner.done.notify_all();

                let guard = _inner.queue.lock().unwrap();
                _inner.alarm.wait(guard).unwrap();

                {
                    *_inner.waiting.lock().unwrap() -= 1;
                }
            });
        }
    }

    pub fn enqueue(&self, task: T) {
        self.inner.queue.lock().unwrap().push_back(task);
        self.inner.alarm.notify_one();
    }

    pub fn join(&self) {
        let waiting = *self.inner.waiting.lock().unwrap();
        if waiting == self.inner.thread_count {
            return;
        }
        loop {
            let guard = self.inner.waiting.lock().unwrap();
            match self
                .inner
                .done
                // TODO: this timeout shouldn't be necessary, but there is some rare condition
                // where this doesn't wake up even though everything is done? Look into it
                .wait_timeout(guard, std::time::Duration::from_millis(10))
            {
                Ok((r, _)) => {
                    if *r == self.inner.thread_count {
                        break;
                    }
                }
                Err(_) => {
                    break;
                }
            }
        }
    }
}
