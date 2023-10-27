use elkodon_bb_lock_free::spsc::index_queue::*;
use elkodon_bb_posix::barrier::{BarrierBuilder, BarrierHandle};
use elkodon_bb_testing::assert_that;
use std::sync::{Arc, Mutex};
use std::thread;

#[test]
fn spsc_index_queue_push_works_until_full() {
    const CAPACITY: usize = 128;
    let sut = FixedSizeIndexQueue::<CAPACITY>::new();

    assert_that!(sut.capacity(), eq CAPACITY);
    assert_that!(sut, len 0);
    assert_that!(sut.is_full(), eq false);
    assert_that!(sut, is_empty);

    let mut sut_producer = sut.acquire_producer().unwrap();

    for i in 0..CAPACITY {
        assert_that!(sut, len i);
        assert_that!(sut_producer.push(i), eq true);
    }
    assert_that!(sut_producer.push(1234), eq false);

    assert_that!(sut.capacity(), eq CAPACITY);
    assert_that!(sut, len CAPACITY);
    assert_that!(sut.is_full(), eq true);
    assert_that!(sut, is_not_empty);
}

#[test]
fn spsc_index_queue_pop_works_until_empty() {
    const CAPACITY: usize = 128;
    let sut = FixedSizeIndexQueue::<CAPACITY>::new();
    let mut sut_producer = sut.acquire_producer().unwrap();
    for i in 0..CAPACITY {
        assert_that!(sut_producer.push(i), eq true);
    }

    assert_that!(sut.capacity(), eq CAPACITY);
    assert_that!(sut.is_full(), eq true);
    assert_that!(sut, is_not_empty);
    assert_that!(sut, len CAPACITY);

    let mut sut_consumer = sut.acquire_consumer().unwrap();
    for i in 0..CAPACITY {
        assert_that!(sut, len CAPACITY - i);
        let result = sut_consumer.pop();
        assert_that!(result, is_some);
        assert_that!(result.unwrap(), eq i);
    }
    assert_that!(sut_consumer.pop(), is_none);

    assert_that!(sut, len 0);
    assert_that!(sut.capacity(), eq CAPACITY);
    assert_that!(sut.is_full(), eq false);
    assert_that!(sut, is_empty);
}

#[test]
fn spsc_index_queue_push_pop_alteration_works() {
    const CAPACITY: usize = 128;
    let sut = FixedSizeIndexQueue::<CAPACITY>::new();
    let mut sut_producer = sut.acquire_producer().unwrap();
    let mut sut_consumer = sut.acquire_consumer().unwrap();

    for i in 0..CAPACITY - 1 {
        assert_that!(sut_producer.push(i), eq true);
        assert_that!(sut_producer.push(i), eq true);

        assert_that!(sut_consumer.pop(), eq Some(i / 2))
    }
}

#[test]
fn spsc_index_queue_get_consumer_twice_fails() {
    let sut = FixedSizeIndexQueue::<1024>::new();
    let _consumer = sut.acquire_consumer().unwrap();
    assert_that!(sut.acquire_consumer(), is_none);
}

#[test]
fn spsc_index_queue_get_consumer_after_release_succeeds() {
    let sut = FixedSizeIndexQueue::<1024>::new();
    {
        let _consumer = sut.acquire_consumer();
    }
    assert_that!(sut.acquire_consumer(), is_some);
}

#[test]
fn spsc_index_queue_get_producer_twice_fails() {
    let sut = FixedSizeIndexQueue::<1024>::new();
    let _producer = sut.acquire_producer().unwrap();
    assert_that!(sut.acquire_producer(), is_none);
}

#[test]
fn spsc_index_queue_get_producer_after_release_succeeds() {
    let sut = FixedSizeIndexQueue::<1024>::new();
    {
        let _producer = sut.acquire_producer();
    }
    assert_that!(sut.acquire_producer(), is_some);
}

#[test]
fn spsc_index_queue_push_pop_works_concurrently() {
    const LIMIT: usize = 1000000;
    const CAPACITY: usize = 1024;

    let sut = FixedSizeIndexQueue::<CAPACITY>::new();
    let mut sut_producer = sut.acquire_producer().unwrap();
    let mut sut_consumer = sut.acquire_consumer().unwrap();

    let storage = Arc::new(Mutex::<Vec<usize>>::new(vec![]));
    let storage_pop = Arc::clone(&storage);
    let handle = BarrierHandle::new();
    let barrier = BarrierBuilder::new(2)
        .is_interprocess_capable(false)
        .create(&handle)
        .unwrap();

    thread::scope(|s| {
        s.spawn(|| {
            let mut counter: usize = 0;
            barrier.wait();
            while counter <= LIMIT {
                if sut_producer.push(counter) {
                    counter += 1;
                }
            }
        });

        s.spawn(|| {
            let mut guard = storage_pop.lock().unwrap();
            barrier.wait();
            loop {
                match sut_consumer.pop() {
                    Some(v) => {
                        guard.push(v);
                        if v == LIMIT {
                            return;
                        }
                    }
                    None => (),
                }
            }
        });
    });

    let guard = storage.lock().unwrap();
    for i in 0..LIMIT {
        assert_that!(guard[i], eq i);
    }
}
