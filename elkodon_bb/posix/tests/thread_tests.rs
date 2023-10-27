use elkodon_bb_posix::clock::*;
use elkodon_bb_posix::thread::*;
use elkodon_bb_testing::assert_that;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::sync::Barrier;
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_millis(25);
static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn thread_set_name_works() {
    let name = ThreadName::from(b"oh-a-thread");
    COUNTER.store(0, Ordering::Relaxed);
    let thread = ThreadBuilder::new()
        .name(&name)
        .spawn(move || {
            nanosleep(TIMEOUT).ok();
            let handle = ThreadHandle::from_self();
            assert_that!(handle.get_name().unwrap(), eq b"oh-a-thread");
            COUNTER.store(1, Ordering::Relaxed);
        })
        .unwrap();

    nanosleep(Duration::from_millis(10)).ok();
    assert_that!(thread.get_name().unwrap(), eq b"oh-a-thread");
    drop(thread);
    assert_that!(COUNTER.load(Ordering::Relaxed), eq 1);
}

#[test]
fn thread_creation_does_not_block() {
    let barrier = Arc::new(Barrier::new(2));
    let barrier_thread = barrier.clone();
    let thread = ThreadBuilder::new()
        .spawn(move || {
            barrier_thread.wait();
        })
        .unwrap();
    barrier.wait();
    drop(thread);
}

#[test]
fn thread_affinity_is_at_least_core_zero() {
    let thread = ThreadBuilder::new()
        .spawn(|| {
            nanosleep(TIMEOUT).ok();
            let handle = ThreadHandle::from_self();
            let affinity = handle.get_affinity().unwrap();
            assert_that!(affinity, is_not_empty);
            assert_that!(affinity[0], eq 0);
        })
        .unwrap();

    let affinity = thread.get_affinity().unwrap();
    assert_that!(affinity, is_not_empty);
    assert_that!(affinity[0], eq 0);
}

#[test]
fn thread_set_affinity_on_creation_works() {
    let thread = ThreadBuilder::new()
        .affinity(0)
        .spawn(|| {
            nanosleep(TIMEOUT).ok();
            let handle = ThreadHandle::from_self();
            let affinity = handle.get_affinity().unwrap();
            assert_that!(affinity, len 1);
            assert_that!(affinity[0], eq 0);
        })
        .unwrap();

    let affinity = thread.get_affinity().unwrap();
    assert_that!(affinity, len 1);
    assert_that!(affinity[0], eq 0);
}

#[test]
fn thread_set_affinity_from_handle_works() {
    let thread = ThreadBuilder::new()
        .affinity(0)
        .spawn(|| {
            let mut handle = ThreadHandle::from_self();
            handle.set_affinity(0).unwrap();
            let affinity = handle.get_affinity().unwrap();
            assert_that!(affinity, len 1);
            assert_that!(affinity[0], eq 0);
            nanosleep(Duration::from_millis(100)).ok();
        })
        .unwrap();

    nanosleep(TIMEOUT).ok();
    let affinity = thread.get_affinity().unwrap();
    assert_that!(affinity, len 1);
    assert_that!(affinity[0], eq 0);
}

#[test]
fn thread_set_affinity_from_thread_works() {
    let mut thread = ThreadBuilder::new()
        .affinity(0)
        .spawn(|| {
            nanosleep(TIMEOUT).ok();
            let handle = ThreadHandle::from_self();
            let affinity = handle.get_affinity().unwrap();
            assert_that!(affinity, len 1);
            assert_that!(affinity[0], eq 0);
        })
        .unwrap();

    thread.set_affinity(0).unwrap();
    let affinity = thread.get_affinity().unwrap();
    assert_that!(affinity, len 1);
    assert_that!(affinity[0], eq 0);
}

#[test]
fn thread_cancel_works() {
    COUNTER.store(0, Ordering::Relaxed);
    let mut thread = ThreadBuilder::new()
        .affinity(0)
        .spawn(|| {
            COUNTER.store(1, Ordering::Relaxed);
            nanosleep(Duration::from_secs(100000)).ok();
        })
        .unwrap();

    // if the thread is not executed we observe a deadlock here
    while COUNTER.load(Ordering::Relaxed) == 0 {}
    // if cancel wouldn't work we would observe a deadlock here
    thread.cancel();
}

#[test]
fn thread_exit_works() {
    COUNTER.store(0, Ordering::Relaxed);
    ThreadBuilder::new()
        .affinity(0)
        .spawn(move || {
            COUNTER.store(1, Ordering::Relaxed);
            thread_exit();
            // if exit wouldn't work we would observe a deadlock here
            nanosleep(Duration::from_secs(100000)).ok();
        })
        .unwrap();
    assert_that!(COUNTER.load(Ordering::Relaxed), eq 1);
}
