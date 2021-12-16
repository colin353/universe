extern crate pool;

use std::thread;

fn main() {
    let pool = pool::PoolQueue::new(5, |x| {
        println!("did the job: {}", x);
    });
    for i in 0..10 {
        pool.enqueue(i);
    }

    pool.join();
}
