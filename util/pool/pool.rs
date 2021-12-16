use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub mod queue;
pub use queue::PoolQueue;

pub struct ThreadPool<T> {
    threads: Vec<Worker<T>>,
    alarm: mpsc::Receiver<T>,
    pub scheduler: ThreadPoolScheduler<T>,
}

#[derive(Clone)]
pub struct ThreadPoolScheduler<T> {
    sender: mpsc::Sender<Job<T>>,
    in_progress: Arc<AtomicUsize>,
}

impl<T> ThreadPoolScheduler<T>
where
    T: Send + Sync + 'static,
{
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() -> T + Send + 'static,
    {
        let job = Box::new(f);
        self.in_progress.fetch_add(1, Ordering::SeqCst);
        self.sender.send(job).unwrap();
    }
}

impl<T> ThreadPool<T>
where
    T: Send + Sync + 'static,
{
    pub fn new(size: usize) -> Self {
        let mut threads = Vec::with_capacity(size);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let (waker, alarm) = mpsc::channel();
        let in_progress = Arc::new(AtomicUsize::new(0));

        for _ in 0..size {
            threads.push(Worker::new(
                receiver.clone(),
                waker.clone(),
                in_progress.clone(),
            ))
        }

        ThreadPool {
            threads,
            alarm,
            scheduler: ThreadPoolScheduler {
                sender,
                in_progress,
            },
        }
    }

    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() -> T + Send + 'static,
    {
        self.scheduler.execute(f);
    }

    // blocks until at least one job completes
    pub fn block_until_job_completes(&self) -> Option<T> {
        if self.scheduler.in_progress.load(Ordering::SeqCst) > 0 {
            return Some(self.alarm.recv().unwrap());
        }
        None
    }

    pub fn join(&self) -> Vec<T> {
        let mut output = Vec::new();
        while self.scheduler.in_progress.load(Ordering::SeqCst) > 0 {
            output.push(self.alarm.recv().unwrap());
        }

        // All tasks are done, so flush the remaining stuff in the channel
        while let Ok(r) = self.alarm.try_recv() {
            output.push(r);
        }

        output
    }

    pub fn get_in_progress(&self) -> usize {
        self.scheduler.in_progress.load(Ordering::SeqCst)
    }
}

pub struct Worker<T> {
    thread: thread::JoinHandle<()>,
    _mark: std::marker::PhantomData<T>,
}

impl<T> Worker<T>
where
    T: Send + 'static,
{
    fn new(
        receiver: Arc<Mutex<mpsc::Receiver<Job<T>>>>,
        waker: mpsc::Sender<T>,
        in_progress: Arc<AtomicUsize>,
    ) -> Self {
        let thread = thread::spawn(move || loop {
            let job = {
                let r = receiver.lock().unwrap();
                match r.recv() {
                    Ok(job) => job,
                    // If we're looking for a job and we get an error,
                    // it means that the parent thread has shut down, so
                    // we should shut down too.
                    Err(_) => return,
                }
            };
            let result = job.call_box();
            in_progress.fetch_sub(1, Ordering::SeqCst);
            waker.send(result).unwrap();
        });
        Worker {
            thread,
            _mark: std::marker::PhantomData,
        }
    }
}

trait FnBox<T> {
    fn call_box(self: Box<Self>) -> T;
}

impl<T, F: FnOnce() -> T> FnBox<T> for F {
    fn call_box(self: Box<F>) -> T {
        (*self)()
    }
}

type Job<T> = Box<dyn FnBox<T> + Send + 'static>;
