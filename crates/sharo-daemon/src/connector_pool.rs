use std::fmt;
use std::sync::mpsc::{self, Receiver, SyncSender, TrySendError};

#[derive(Debug)]
pub enum PoolError {
    Overloaded,
    Disconnected,
    WorkerFailed,
}

impl fmt::Display for PoolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PoolError::Overloaded => write!(f, "connector_pool_overloaded"),
            PoolError::Disconnected => write!(f, "connector_pool_disconnected"),
            PoolError::WorkerFailed => write!(f, "connector_pool_worker_failed"),
        }
    }
}

type Job = Box<dyn FnOnce() + Send + 'static>;

pub struct TaskHandle<R> {
    rx: Receiver<R>,
}

impl<R> TaskHandle<R> {
    pub fn wait(self) -> Result<R, PoolError> {
        self.rx.recv().map_err(|_| PoolError::WorkerFailed)
    }
}

#[derive(Clone)]
pub struct BlockingPool {
    tx: SyncSender<Job>,
}

impl BlockingPool {
    pub fn new(worker_count: usize, queue_capacity: usize) -> Self {
        let worker_count = worker_count.max(1);
        let queue_capacity = queue_capacity.max(1);
        let (tx, rx) = mpsc::sync_channel::<Job>(queue_capacity);
        let shared_rx = std::sync::Arc::new(std::sync::Mutex::new(rx));

        for _ in 0..worker_count {
            let worker_rx = std::sync::Arc::clone(&shared_rx);
            std::thread::spawn(move || {
                loop {
                    let recv_result = {
                        let guard = worker_rx.lock();
                        match guard {
                            Ok(guard) => guard.recv(),
                            Err(_) => return,
                        }
                    };

                    match recv_result {
                        Ok(job) => job(),
                        Err(_) => return,
                    }
                }
            });
        }

        Self { tx }
    }

    pub fn submit<F, R>(&self, work: F) -> Result<TaskHandle<R>, PoolError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_tx, result_rx) = mpsc::channel::<R>();
        let job: Job = Box::new(move || {
            let _ = result_tx.send(work());
        });

        match self.tx.try_send(job) {
            Ok(()) => Ok(TaskHandle { rx: result_rx }),
            Err(TrySendError::Full(_)) => Err(PoolError::Overloaded),
            Err(TrySendError::Disconnected(_)) => Err(PoolError::Disconnected),
        }
    }

    pub fn execute_with_result<F, R>(&self, work: F) -> Result<R, PoolError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.submit(work)?.wait()
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockingPool, PoolError};

    #[test]
    fn pool_reuses_fixed_workers() {
        let pool = BlockingPool::new(2, 4);
        let a = pool
            .execute_with_result(|| std::thread::current().id())
            .expect("job a");
        let b = pool
            .execute_with_result(|| std::thread::current().id())
            .expect("job b");
        let c = pool
            .execute_with_result(|| std::thread::current().id())
            .expect("job c");
        assert!(a == b || a == c || b == c);
    }

    #[test]
    fn pool_returns_overload_on_full_queue() {
        let pool = BlockingPool::new(1, 1);
        let (start_tx, start_rx) = std::sync::mpsc::channel::<()>();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();

        let first = pool
            .submit(move || {
                let _ = start_tx.send(());
                let _ = release_rx.recv();
                1_u64
            })
            .expect("first task");
        start_rx.recv().expect("worker started");

        let second = pool.submit(|| 2_u64).expect("second enqueued");
        let third = pool.submit(|| 3_u64);
        assert!(matches!(third, Err(PoolError::Overloaded)));

        release_tx.send(()).expect("release worker");
        assert_eq!(first.wait().expect("first result"), 1);
        assert_eq!(second.wait().expect("second result"), 2);
    }
}
