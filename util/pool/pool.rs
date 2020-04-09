use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct ThreadPool {
    threads: Vec<Worker>,
    sender: mpsc::Sender<Job>,
    alarm: mpsc::Receiver<()>,
    in_progress: Arc<AtomicUsize>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        let mut threads = Vec::with_capacity(size);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let (waker, alarm) = mpsc::channel();
        let in_progress = Arc::new(AtomicUsize::new(0));

        for id in 0..size {
            threads.push(Worker::new(
                id,
                receiver.clone(),
                waker.clone(),
                in_progress.clone(),
            ))
        }

        ThreadPool {
            threads,
            sender,
            alarm,
            in_progress,
        }
    }
}

impl ThreadPool {
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.in_progress.fetch_add(1, Ordering::Relaxed);
        self.sender.send(job).unwrap();
    }

    // blocks until at least one job completes
    pub fn block_until_job_completes(&self) {
        if self.in_progress.load(Ordering::Relaxed) > 0 {
            self.alarm.recv().unwrap();
        }
    }

    pub fn join(&self) {
        while self.in_progress.load(Ordering::Relaxed) > 0 {
            self.alarm.recv().unwrap();
        }
    }

    pub fn get_in_progress(&self) -> usize {
        self.in_progress.load(Ordering::Relaxed)
    }
}

pub struct Worker {
    id: usize,
    thread: thread::JoinHandle<()>,
}

impl Worker {
    fn new(
        id: usize,
        receiver: Arc<Mutex<mpsc::Receiver<Job>>>,
        waker: mpsc::Sender<()>,
        in_progress: Arc<AtomicUsize>,
    ) -> Worker {
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
            job.call_box();
            in_progress.fetch_sub(1, Ordering::Relaxed);
            waker.send(()).unwrap();
        });
        Worker { id, thread }
    }
}

trait FnBox {
    fn call_box(self: Box<Self>);
}

impl<F: FnOnce()> FnBox for F {
    fn call_box(self: Box<F>) {
        (*self)()
    }
}

type Job = Box<FnBox + Send + 'static>;
