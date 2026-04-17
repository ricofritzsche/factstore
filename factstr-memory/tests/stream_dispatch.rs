use std::sync::{
    Arc, Condvar, Mutex,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::Duration;

use factstr::{EventStore, NewEvent, StreamHandlerError};
use factstr_memory::MemoryStore;
use serde_json::json;

#[test]
fn blocking_handler_does_not_block_append_completion() {
    let store = MemoryStore::new();
    let (handler_started_sender, handler_started_receiver) = mpsc::channel();
    let release_handler = Arc::new((Mutex::new(false), Condvar::new()));

    let _subscription = store
        .stream_all(Arc::new({
            let release_handler = Arc::clone(&release_handler);
            move |event_records| {
                assert_eq!(event_records.len(), 1);
                let _ = handler_started_sender.send(());

                let (lock, condvar) = &*release_handler;
                let mut released = lock.lock().expect("handler gate lock should succeed");
                while !*released {
                    released = condvar
                        .wait(released)
                        .expect("handler gate wait should succeed");
                }

                Ok::<(), StreamHandlerError>(())
            }
        }))
        .expect("subscribe_all should succeed");

    let (append_done_sender, append_done_receiver) = mpsc::channel();
    let append_thread = thread::spawn(move || {
        let append_result = store.append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )]);
        append_done_sender
            .send(append_result)
            .expect("append result should be sent");
    });

    let append_result = append_done_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("append should finish before the blocking handler is released")
        .expect("append should succeed");
    assert_eq!(append_result.first_sequence_number, 1);

    handler_started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("blocking handler should eventually start");

    let (lock, condvar) = &*release_handler;
    *lock.lock().expect("handler gate lock should succeed") = true;
    condvar.notify_all();

    append_thread.join().expect("append thread should finish");
}

#[test]
fn already_snapshotted_delivery_may_arrive_after_unsubscribe_but_future_commits_do_not() {
    let store = MemoryStore::new();
    let delivered_batches = Arc::new(Mutex::new(Vec::new()));
    let invocation_count = Arc::new(AtomicUsize::new(0));
    let (first_handler_started_sender, first_handler_started_receiver) = mpsc::channel();
    let release_first_handler = Arc::new((Mutex::new(false), Condvar::new()));

    let subscription = store
        .stream_all(Arc::new({
            let delivered_batches = Arc::clone(&delivered_batches);
            let invocation_count = Arc::clone(&invocation_count);
            let release_first_handler = Arc::clone(&release_first_handler);
            move |event_records| {
                delivered_batches
                    .lock()
                    .expect("delivery log lock should succeed")
                    .push(event_records);

                if invocation_count.fetch_add(1, Ordering::SeqCst) == 0 {
                    let _ = first_handler_started_sender.send(());
                    let (lock, condvar) = &*release_first_handler;
                    let mut released = lock.lock().expect("handler gate lock should succeed");
                    while !*released {
                        released = condvar
                            .wait(released)
                            .expect("handler gate wait should succeed");
                    }
                }

                Ok::<(), StreamHandlerError>(())
            }
        }))
        .expect("subscribe_all should succeed");

    store
        .append(vec![NewEvent::new(
            "account-opened",
            json!({ "accountId": "a1" }),
        )])
        .expect("first append should succeed");

    first_handler_started_receiver
        .recv_timeout(Duration::from_secs(1))
        .expect("first delivery should start and block");

    store
        .append(vec![NewEvent::new(
            "account-credited",
            json!({ "accountId": "a1" }),
        )])
        .expect("second append should succeed while the first handler is blocked");

    subscription.unsubscribe();

    store
        .append(vec![NewEvent::new(
            "account-closed",
            json!({ "accountId": "a1" }),
        )])
        .expect("third append should succeed after unsubscribe");

    let (lock, condvar) = &*release_first_handler;
    *lock.lock().expect("handler gate lock should succeed") = true;
    condvar.notify_all();

    let deadline = std::time::Instant::now() + Duration::from_secs(1);
    loop {
        let delivered_batches = delivered_batches
            .lock()
            .expect("delivery log lock should succeed")
            .clone();

        if delivered_batches.len() == 2 {
            let delivered_sequences = delivered_batches
                .iter()
                .map(|batch| batch[0].sequence_number)
                .collect::<Vec<_>>();
            assert_eq!(delivered_sequences, vec![1, 2]);
            return;
        }

        assert!(
            std::time::Instant::now() < deadline,
            "expected exactly two delivered batches after unsubscribe, got {}",
            delivered_batches.len()
        );

        thread::sleep(Duration::from_millis(10));
    }
}
