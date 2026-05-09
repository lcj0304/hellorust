mod threadpool;
use threadpool::ThreadPool;

fn main() {
    println!("Hello, world!");
    let result = add(5, 3);
    println!("The sum of 5 and 3 is: {}", result);
    let result = subtract(5, 3);
    println!("The difference of 5 and 3 is: {}", result);

    let pool = ThreadPool::new(4);

    for i in 0..8 {
        pool.execute(move || {
            println!("Task {} is running", i);
        });
    }
}

pub fn add(a: i32, b: i32) -> i32 {
    a + b
}

pub fn subtract(a: i32, b: i32) -> i32 {
    a - b
}
