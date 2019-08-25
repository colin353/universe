extern crate pool;

use std::thread;

fn main() {
    let threadpool = pool::ThreadPool::new(5);
    for i in 0..10 {
        threadpool.execute(move || {
            println!("iteration {}", i);
        });
    }

    thread::park();
}
