use std::sync::{mpsc, Arc, Mutex};
use std::thread;

type Job = Box<dyn FnOnce() + Send + 'static>;

/// A fixed-size pool of worker threads consuming jobs from a shared channel.
pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

impl ThreadPool {
    /// Creates a pool with `size` worker threads.
    ///
    /// # Panics
    ///
    /// Panics if `size` is zero.
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0, "thread pool size must be greater than zero");

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));

        let workers = (0..size)
            .map(|id| Worker::new(id, Arc::clone(&receiver)))
            .collect();

        ThreadPool {
            workers,
            sender: Some(sender),
        }
    }

    /// Sends a job to be executed by one of the workers.
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.sender
            .as_ref()
            .expect("thread pool is shut down")
            .send(Box::new(f))
            .expect("all workers have died");
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        // Closing the channel makes every worker's recv() fail, so they exit.
        drop(self.sender.take());

        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().expect("worker thread panicked");
            }
        }
    }
}

struct Worker {
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Worker {
        let thread = thread::Builder::new()
            .name(format!("worker-{id}"))
            .spawn(move || loop {
                let job = receiver.lock().expect("pool mutex poisoned").recv();
                match job {
                    Ok(job) => job(),
                    Err(_) => break, // channel closed: pool is shutting down
                }
            })
            .expect("failed to spawn worker thread");

        Worker {
            thread: Some(thread),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn executes_all_jobs() {
        let counter = Arc::new(AtomicUsize::new(0));
        {
            let pool = ThreadPool::new(4);
            for _ in 0..100 {
                let counter = Arc::clone(&counter);
                pool.execute(move || {
                    counter.fetch_add(1, Ordering::SeqCst);
                });
            }
            // Drop waits for all workers to finish.
        }
        assert_eq!(counter.load(Ordering::SeqCst), 100);
    }

    #[test]
    #[should_panic]
    fn zero_size_panics() {
        ThreadPool::new(0);
    }
}
