extern crate pool;

use std::thread;

fn main() {
    let pool = pool::ThreadPool::new(5);
    for i in 0..10 {
        pool.execute(move || {
            println!("iteration {}", i);
            i
        });
    }

    let result = pool.join();
    println!("result: {:?}", result);
}
