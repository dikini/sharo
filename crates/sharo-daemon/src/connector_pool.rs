use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender, TrySendError, bounded};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PoolPolicy {
    pub min_threads: usize,
    pub max_threads: usize,
    pub queue_capacity: usize,
    pub scale_up_queue_threshold: usize,
    pub scale_down_idle_ms: u64,
    pub cooldown_ms: u64,
}

pub struct TaskHandle<R> {
    rx: std::sync::mpsc::Receiver<R>,
}

impl<R> TaskHandle<R> {
    pub fn wait(self) -> Result<R, PoolError> {
        self.rx.recv().map_err(|_| PoolError::WorkerFailed)
    }
}

struct PoolState {
    policy: PoolPolicy,
    rx: Receiver<Job>,
    pending_jobs: AtomicUsize,
    active_workers: AtomicUsize,
    last_scale_event: Mutex<Instant>,
    #[cfg(test)]
    test_hooks: TestHooks,
}

#[derive(Clone)]
pub struct BlockingPool {
    tx: Sender<Job>,
    state: Arc<PoolState>,
}

impl BlockingPool {
    pub fn new(policy: PoolPolicy) -> Self {
        #[cfg(test)]
        let pool = Self::new_with_test_hooks(policy, TestHooks::default());

        #[cfg(not(test))]
        let pool = {
            let policy = normalized_policy(policy);
            let (tx, rx) = bounded::<Job>(policy.queue_capacity);
            let state = Arc::new(PoolState {
                policy: policy.clone(),
                rx,
                pending_jobs: AtomicUsize::new(0),
                active_workers: AtomicUsize::new(0),
                last_scale_event: Mutex::new(
                    Instant::now()
                        .checked_sub(Duration::from_millis(policy.cooldown_ms))
                        .unwrap_or_else(Instant::now),
                ),
            });

            let pool = Self { tx, state };
            for _ in 0..policy.min_threads {
                pool.spawn_worker();
            }
            pool
        };

        pool
    }

    #[cfg(test)]
    fn new_with_test_hooks(policy: PoolPolicy, test_hooks: TestHooks) -> Self {
        let policy = normalized_policy(policy);
        let (tx, rx) = bounded::<Job>(policy.queue_capacity);
        let state = Arc::new(PoolState {
            policy: policy.clone(),
            rx,
            pending_jobs: AtomicUsize::new(0),
            active_workers: AtomicUsize::new(0),
            last_scale_event: Mutex::new(
                Instant::now()
                    .checked_sub(Duration::from_millis(policy.cooldown_ms))
                    .unwrap_or_else(Instant::now),
            ),
            #[cfg(test)]
            test_hooks,
        });

        let pool = Self { tx, state };
        for _ in 0..policy.min_threads {
            pool.spawn_worker();
        }
        pool
    }

    pub fn submit<F, R>(&self, work: F) -> Result<TaskHandle<R>, PoolError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_tx, result_rx) = std::sync::mpsc::channel::<R>();
        let job: Job = Box::new(move || {
            let _ = result_tx.send(work());
        });

        self.state.pending_jobs.fetch_add(1, Ordering::SeqCst);
        match self.tx.try_send(job) {
            Ok(()) => {
                self.maybe_scale_up();
                Ok(TaskHandle { rx: result_rx })
            }
            Err(TrySendError::Full(_)) => {
                self.state.pending_jobs.fetch_sub(1, Ordering::SeqCst);
                Err(PoolError::Overloaded)
            }
            Err(TrySendError::Disconnected(_)) => {
                self.state.pending_jobs.fetch_sub(1, Ordering::SeqCst);
                Err(PoolError::Disconnected)
            }
        }
    }

    pub fn execute_with_result<F, R>(&self, work: F) -> Result<R, PoolError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        self.submit(work)?.wait()
    }

    #[cfg(test)]
    pub fn current_worker_count(&self) -> usize {
        self.state.active_workers.load(Ordering::SeqCst)
    }

    fn maybe_scale_up(&self) {
        let pending = self.state.pending_jobs.load(Ordering::SeqCst);
        if pending < self.state.policy.scale_up_queue_threshold {
            return;
        }

        let active = self.state.active_workers.load(Ordering::SeqCst);
        if active >= self.state.policy.max_threads {
            return;
        }

        if !self.try_mark_scale_event() {
            return;
        }

        #[cfg(test)]
        self.run_before_spawn_hook();

        if !self.try_reserve_worker_slot() {
            return;
        }

        self.spawn_reserved_worker();
    }

    fn spawn_worker(&self) {
        if !self.try_reserve_worker_slot() {
            return;
        }
        self.spawn_reserved_worker();
    }

    fn spawn_reserved_worker(&self) {
        let state = Arc::clone(&self.state);
        if std::thread::Builder::new()
            .name("sharo-connector-worker".to_string())
            .spawn(move || {
                let idle_timeout = Duration::from_millis(state.policy.scale_down_idle_ms);
                loop {
                    match state.rx.recv_timeout(idle_timeout) {
                        Ok(job) => {
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                            state.pending_jobs.fetch_sub(1, Ordering::SeqCst);
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                            if state.active_workers.load(Ordering::SeqCst)
                                > state.policy.min_threads
                                && try_mark_scale_event(&state)
                            {
                                state.active_workers.fetch_sub(1, Ordering::SeqCst);
                                return;
                            }
                        }
                        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                            state.active_workers.fetch_sub(1, Ordering::SeqCst);
                            return;
                        }
                    }
                }
            })
            .is_err()
        {
            self.release_worker_slot();
        }
    }

    fn try_mark_scale_event(&self) -> bool {
        try_mark_scale_event(&self.state)
    }

    fn try_reserve_worker_slot(&self) -> bool {
        let max_threads = self.state.policy.max_threads;
        let mut active = self.state.active_workers.load(Ordering::SeqCst);
        loop {
            if active >= max_threads {
                return false;
            }
            match self.state.active_workers.compare_exchange_weak(
                active,
                active + 1,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => return true,
                Err(current) => active = current,
            }
        }
    }

    fn release_worker_slot(&self) {
        self.state.active_workers.fetch_sub(1, Ordering::SeqCst);
    }

    #[cfg(test)]
    fn run_before_spawn_hook(&self) {
        self.state.test_hooks.run_before_spawn();
    }
}

fn normalized_policy(mut policy: PoolPolicy) -> PoolPolicy {
    policy.min_threads = policy.min_threads.max(1);
    policy.max_threads = policy.max_threads.max(policy.min_threads);
    policy.queue_capacity = policy.queue_capacity.max(1);
    policy.scale_up_queue_threshold = policy.scale_up_queue_threshold.max(1);
    policy.scale_down_idle_ms = policy.scale_down_idle_ms.max(1);
    policy.cooldown_ms = policy.cooldown_ms.max(1);
    policy
}

fn try_mark_scale_event(state: &Arc<PoolState>) -> bool {
    let now = Instant::now();
    let mut guard = match state.last_scale_event.lock() {
        Ok(g) => g,
        Err(_) => return false,
    };
    if now.duration_since(*guard).as_millis() < u128::from(state.policy.cooldown_ms) {
        return false;
    }
    *guard = now;
    true
}

#[cfg(test)]
#[derive(Clone, Default)]
struct TestHooks {
    before_spawn: Option<Arc<dyn Fn() + Send + Sync + 'static>>,
}

#[cfg(test)]
impl TestHooks {
    fn run_before_spawn(&self) {
        if let Some(hook) = &self.before_spawn {
            hook();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BlockingPool, PoolError, PoolPolicy, TestHooks};
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) {
        let start = std::time::Instant::now();
        while start.elapsed() < timeout {
            if predicate() {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("condition was not met within timeout");
    }

    #[test]
    fn pool_reuses_fixed_workers() {
        let pool = BlockingPool::new(PoolPolicy {
            min_threads: 2,
            max_threads: 2,
            queue_capacity: 4,
            scale_up_queue_threshold: 2,
            scale_down_idle_ms: 1_000,
            cooldown_ms: 1_000,
        });
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
        assert_eq!(pool.current_worker_count(), 2);
    }

    #[test]
    fn pool_returns_overload_on_full_queue() {
        let pool = BlockingPool::new(PoolPolicy {
            min_threads: 1,
            max_threads: 1,
            queue_capacity: 1,
            scale_up_queue_threshold: 10,
            scale_down_idle_ms: 1_000,
            cooldown_ms: 1_000,
        });
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

    #[test]
    fn scale_up_respects_threshold_and_cooldown() {
        let pool = BlockingPool::new(PoolPolicy {
            min_threads: 1,
            max_threads: 3,
            queue_capacity: 32,
            scale_up_queue_threshold: 1,
            scale_down_idle_ms: 5_000,
            cooldown_ms: 1,
        });
        assert_eq!(pool.current_worker_count(), 1);

        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let held = pool
            .submit(move || {
                let _ = release_rx.recv();
                1_u64
            })
            .expect("held task");

        wait_until(Duration::from_millis(500), || {
            pool.current_worker_count() == 2
        });

        let _another = pool.submit(|| 2_u64).expect("another task");
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(pool.current_worker_count(), 2);

        release_tx.send(()).expect("release held task");
        let _ = held.wait().expect("held result");
    }

    #[test]
    fn scale_down_respects_idle_window() {
        let pool = BlockingPool::new(PoolPolicy {
            min_threads: 1,
            max_threads: 3,
            queue_capacity: 32,
            scale_up_queue_threshold: 1,
            scale_down_idle_ms: 80,
            cooldown_ms: 1,
        });

        let _ = pool.execute_with_result(|| 42_u64).expect("initial job");
        wait_until(Duration::from_millis(400), || {
            pool.current_worker_count() >= 2
        });
        wait_until(Duration::from_millis(2_000), || {
            pool.current_worker_count() == 1
        });
    }

    #[test]
    fn scale_state_always_within_min_max_bounds() {
        let policy = PoolPolicy {
            min_threads: 2,
            max_threads: 4,
            queue_capacity: 128,
            scale_up_queue_threshold: 2,
            scale_down_idle_ms: 50,
            cooldown_ms: 10,
        };
        let pool = BlockingPool::new(policy.clone());

        for _ in 0..64 {
            let _ = pool.submit(|| {
                std::thread::sleep(Duration::from_millis(2));
                1_u64
            });
        }

        wait_until(Duration::from_millis(1_500), || {
            let w = pool.current_worker_count();
            w >= policy.min_threads && w <= policy.max_threads
        });

        let workers = pool.current_worker_count();
        assert!(workers >= policy.min_threads);
        assert!(workers <= policy.max_threads);
    }

    #[test]
    fn large_cooldown_does_not_panic_on_startup() {
        let _pool = BlockingPool::new(PoolPolicy {
            min_threads: 1,
            max_threads: 2,
            queue_capacity: 8,
            scale_up_queue_threshold: 1,
            scale_down_idle_ms: 100,
            cooldown_ms: u64::MAX,
        });
    }

    #[test]
    fn fast_jobs_do_not_underflow_pending_counter() {
        let pool = BlockingPool::new(PoolPolicy {
            min_threads: 1,
            max_threads: 2,
            queue_capacity: 64,
            scale_up_queue_threshold: 1,
            scale_down_idle_ms: 500,
            cooldown_ms: 10,
        });

        for _ in 0..200 {
            let handle = pool.submit(|| 1_u64).expect("submit");
            assert_eq!(handle.wait().expect("wait"), 1_u64);
        }

        wait_until(Duration::from_millis(200), || {
            pool.current_worker_count() >= 1
        });
        assert!(pool.current_worker_count() <= 2);
    }

    #[test]
    fn scale_up_reservation_is_atomic_under_race() {
        let hook_calls = Arc::new(AtomicUsize::new(0));
        let pool = BlockingPool::new_with_test_hooks(
            PoolPolicy {
                min_threads: 1,
                max_threads: 2,
                queue_capacity: 8,
                scale_up_queue_threshold: 2,
                scale_down_idle_ms: 5_000,
                cooldown_ms: 1,
            },
            TestHooks {
                before_spawn: Some({
                    let hook_calls = Arc::clone(&hook_calls);
                    Arc::new(move || {
                        let call = hook_calls.fetch_add(1, Ordering::SeqCst);
                        if call == 1 {
                            std::thread::sleep(Duration::from_millis(100));
                        }
                    })
                }),
            },
        );
        pool.state.pending_jobs.store(2, Ordering::SeqCst);

        let pool_a = pool.clone();
        let submit_a = std::thread::spawn(move || {
            pool_a.maybe_scale_up();
        });

        std::thread::sleep(Duration::from_millis(20));

        let pool_b = pool.clone();
        let submit_b = std::thread::spawn(move || {
            pool_b.maybe_scale_up();
        });

        submit_a.join().expect("join submit a");
        submit_b.join().expect("join submit b");

        wait_until(Duration::from_millis(500), || {
            pool.current_worker_count() == 2
        });
        assert_eq!(pool.current_worker_count(), 2);
    }

    #[test]
    fn worker_count_never_exceeds_max_threads_under_parallel_submit() {
        for _ in 0..4 {
            let hook_calls = Arc::new(AtomicUsize::new(0));
            let pool = BlockingPool::new_with_test_hooks(
                PoolPolicy {
                    min_threads: 1,
                    max_threads: 2,
                    queue_capacity: 8,
                    scale_up_queue_threshold: 2,
                    scale_down_idle_ms: 5_000,
                    cooldown_ms: 1,
                },
                TestHooks {
                    before_spawn: Some({
                        let hook_calls = Arc::clone(&hook_calls);
                        Arc::new(move || {
                            let call = hook_calls.fetch_add(1, Ordering::SeqCst);
                            if call == 1 {
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        })
                    }),
                },
            );
            pool.state.pending_jobs.store(2, Ordering::SeqCst);

            let mut submitters = Vec::new();
            for delay_ms in [0_u64, 5_u64] {
                let pool = pool.clone();
                submitters.push(std::thread::spawn(move || {
                    std::thread::sleep(Duration::from_millis(delay_ms));
                    pool.maybe_scale_up();
                }));
            }

            for submitter in submitters {
                submitter.join().expect("join submitter");
            }

            wait_until(Duration::from_millis(500), || {
                pool.current_worker_count() == 2
            });
            assert!(pool.current_worker_count() <= 2);
        }
    }
}
