//! 一个简单、符合 Rust 惯用风格的线程池实现。
//!
//! # 示例
//!
//! ```
//! use helloworld::threadpool::ThreadPool;
//!
//! let pool = ThreadPool::new(4);
//!
//! for i in 0..8 {
//!     pool.execute(move || {
//!         println!("任务 {} 正在执行", i);
//!     });
//! }
//! // 线程池在离开作用域时自动关闭
//! ```

use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

/// 任务类型：一个可发送的、只调用一次的闭包
type Job = Box<dyn FnOnce() + Send + 'static>;

/// 消息类型：要么是任务，要么是终止信号
enum Message {
    /// 需要执行的任务
    NewJob(Job),
    /// 通知 worker 线程退出
    Terminate,
}

/// 线程池，管理一组工作线程并通过通道分发任务。
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: mpsc::Sender<Message>,
}

impl ThreadPool {
    /// 创建一个包含 `size` 个工作线程的新线程池。
    ///
    /// # Panics
    ///
    /// 当 `size` 为 0 时会 panic。
    ///
    /// # 示例
    ///
    /// ```
    /// use helloworld::threadpool::ThreadPool;
    /// let pool = ThreadPool::new(4);
    /// ```
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "线程池大小必须大于 0");

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let workers = (0..size)
            .map(|id| Worker::new(id, Arc::clone(&receiver)))
            .collect();

        ThreadPool { workers, sender }
    }

    /// 向线程池提交一个闭包任务。
    ///
    /// 被提交的闭包会在某个工作线程中执行。
    /// 如果线程池正在关闭，任务可能不会被接收（但不会 panic）。
    ///
    /// # 示例
    ///
    /// ```
    /// use helloworld::threadpool::ThreadPool;
    /// let pool = ThreadPool::new(4);
    /// pool.execute(|| println!("hello"));
    /// ```
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender
            .send(Message::NewJob(job))
            .expect("工作线程已全部终止，无法发送任务");
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        println!("正在关闭线程池，通知所有 worker 退出...");

        // 向每个 worker 发送终止信号
        for _ in &self.workers {
            self.sender
                .send(Message::Terminate)
                .expect("发送终止信号失败");
        }

        // 等待所有工作线程结束
        for worker in &mut self.workers {
            println!("正在关闭 worker {}", worker.id);

            if let Some(handle) = worker.handle.take() {
                handle
                    .join()
                    .unwrap_or_else(|_| eprintln!("worker {} join 时发生 panic", worker.id));
            }
        }

        println!("线程池已关闭");
    }
}

/// 工作线程封装，持有线程句柄和 ID。
struct Worker {
    id: usize,
    handle: Option<JoinHandle<()>>,
}

impl Worker {
    /// 创建一个新的 Worker，从共享接收器中循环接收消息。
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Message>>>) -> Worker {
        let handle = thread::Builder::new()
            .name(format!("线程池-worker-{}", id))
            .spawn(move || {
                loop {
                    // 等待队列中的下一个消息
                    let message = receiver
                        .lock()
                        .expect("Worker 获取锁失败（Mutex 可能已中毒）")
                        .recv()
                        .expect("所有发送端已关闭，接收失败");

                    match message {
                        Message::NewJob(job) => {
                            println!("worker {} 收到任务，正在执行...", id);
                            job();
                        }
                        Message::Terminate => {
                            println!("worker {} 收到终止信号，退出中...", id);
                            break;
                        }
                    }
                }
            })
            .expect("线程创建失败");

        Worker {
            id,
            handle: Some(handle),
        }
    }
}
