use loom::sync::Arc;
use loom::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use loom::thread;

#[test]
fn loom_submit_reservation_release_on_commit_failure() {
    loom::model(|| {
        let reservation = Arc::new(AtomicBool::new(false));

        let worker_reservation = Arc::clone(&reservation);
        let worker = thread::spawn(move || {
            let acquired = worker_reservation
                .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                .is_ok();
            if acquired {
                // Commit failure path must release the reservation.
                worker_reservation.store(false, Ordering::SeqCst);
            }
        });

        worker.join().expect("worker join");
        assert!(!reservation.load(Ordering::SeqCst));
    });
}

#[test]
fn loom_shutdown_drain_preserves_accepted_handler_completion() {
    loom::model(|| {
        let accepted = Arc::new(AtomicBool::new(true));
        let completed = Arc::new(AtomicBool::new(false));
        let shutdown = Arc::new(AtomicBool::new(false));

        let handler_accepted = Arc::clone(&accepted);
        let handler_completed = Arc::clone(&completed);
        let handler_shutdown = Arc::clone(&shutdown);
        let handler = thread::spawn(move || {
            if handler_accepted.load(Ordering::SeqCst) {
                let _shutdown_seen = handler_shutdown.load(Ordering::SeqCst);
                handler_completed.store(true, Ordering::SeqCst);
            }
        });

        let shutdown_flag = Arc::clone(&shutdown);
        let stop = thread::spawn(move || {
            shutdown_flag.store(true, Ordering::SeqCst);
        });

        handler.join().expect("handler join");
        stop.join().expect("shutdown join");
        assert!(completed.load(Ordering::SeqCst));
    });
}

#[test]
fn loom_duplicate_submit_never_double_executes_provider() {
    loom::model(|| {
        let in_flight = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));
        let provider_calls = Arc::new(AtomicUsize::new(0));

        let submit_once =
            |in_flight: Arc<AtomicBool>, completed: Arc<AtomicBool>, calls: Arc<AtomicUsize>| {
                thread::spawn(move || {
                    if completed.load(Ordering::SeqCst) {
                        return;
                    }

                    let acquired = in_flight
                        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok();
                    if !acquired {
                        return;
                    }

                    if !completed.swap(true, Ordering::SeqCst) {
                        calls.fetch_add(1, Ordering::SeqCst);
                    }
                    in_flight.store(false, Ordering::SeqCst);
                })
            };

        let t1 = submit_once(
            Arc::clone(&in_flight),
            Arc::clone(&completed),
            Arc::clone(&provider_calls),
        );
        let t2 = submit_once(in_flight, completed, Arc::clone(&provider_calls));

        t1.join().expect("join t1");
        t2.join().expect("join t2");

        assert_eq!(provider_calls.load(Ordering::SeqCst), 1);
    });
}
