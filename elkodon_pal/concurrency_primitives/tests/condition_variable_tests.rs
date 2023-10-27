use std::{
    hint::spin_loop,
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use elkodon_bb_testing::assert_that;
use elkodon_pal_concurrency_primitives::{barrier::Barrier, condition_variable::*};

const TIMEOUT: Duration = Duration::from_millis(25);

#[test]
fn condition_variable_notify_one_unblocks_one() {
    const NUMBER_OF_THREADS: u32 = 3;
    let barrier = Barrier::new(NUMBER_OF_THREADS + 1);
    let sut = ConditionVariable::new();
    let mtx = Mutex::new();
    let counter = AtomicU32::new(0);
    let triggered_thread = AtomicU32::new(0);

    std::thread::scope(|s| {
        s.spawn(|| {
            barrier.wait(|_, _| {}, |_| {});
            mtx.lock(|_, _| true);
            assert_that!(sut.wait(
                &mtx,
                |_| {},
                |_, _| {
                    while triggered_thread.load(Ordering::Relaxed) < 1 {
                        spin_loop()
                    }
                    true
                },
                |_, _| true
            ), eq true);
            counter.fetch_add(1, Ordering::Relaxed);
            mtx.unlock(|_| {});
        });

        s.spawn(|| {
            barrier.wait(|_, _| {}, |_| {});
            mtx.lock(|_, _| true);
            assert_that!(sut.wait(
                &mtx,
                |_| {},
                |_, _| {
                    while triggered_thread.load(Ordering::Relaxed) < 2 {
                        spin_loop()
                    }
                    true
                },
                |_, _| true
            ), eq true);
            counter.fetch_add(1, Ordering::Relaxed);
            mtx.unlock(|_| {});
        });

        s.spawn(|| {
            barrier.wait(|_, _| {}, |_| {});
            mtx.lock(|_, _| true);
            assert_that!(sut.wait(
                &mtx,
                |_| {},
                |_, _| {
                    while triggered_thread.load(Ordering::Relaxed) < 3 {
                        spin_loop()
                    }
                    true
                },
                |_, _| true
            ), eq true);
            counter.fetch_add(1, Ordering::Relaxed);
            mtx.unlock(|_| {});
        });

        barrier.wait(|_, _| {}, |_| {});
        std::thread::sleep(TIMEOUT);
        assert_that!(counter.load(Ordering::Relaxed), eq 0);

        for i in 0..NUMBER_OF_THREADS {
            sut.notify(|_| {
                triggered_thread.fetch_add(1, Ordering::Relaxed);
            });
            std::thread::sleep(TIMEOUT);
            assert_that!(counter.load(Ordering::Relaxed), eq i + 1);
        }
    });
}

#[test]
fn condition_variable_notify_all_unblocks_all() {
    const NUMBER_OF_THREADS: u32 = 5;
    let barrier = Barrier::new(NUMBER_OF_THREADS + 1);
    let sut = ConditionVariable::new();
    let mtx = Mutex::new();
    let counter = AtomicU32::new(0);
    let triggered_thread = AtomicU32::new(0);

    std::thread::scope(|s| {
        for _ in 0..NUMBER_OF_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                mtx.lock(|_, _| true);
                assert_that!(sut.wait(
                    &mtx,
                    |_| {},
                    |_, _| {
                        while triggered_thread.load(Ordering::Relaxed) < 1 {
                            spin_loop()
                        }
                        true
                    },
                    |_, _| true
                ), eq true);
                counter.fetch_add(1, Ordering::Relaxed);
                mtx.unlock(|_| {});
            });
        }

        barrier.wait(|_, _| {}, |_| {});
        std::thread::sleep(TIMEOUT);
        assert_that!(counter.load(Ordering::Relaxed), eq 0);

        sut.notify(|_| {
            triggered_thread.fetch_add(1, Ordering::Relaxed);
        });
        std::thread::sleep(TIMEOUT);
        assert_that!(counter.load(Ordering::Relaxed), eq NUMBER_OF_THREADS);
    });
}

#[test]
fn condition_variable_mutex_is_locked_when_wait_returns() {
    const NUMBER_OF_THREADS: u32 = 5;
    let barrier = Barrier::new(NUMBER_OF_THREADS + 1);
    let sut = ConditionVariable::new();
    let mtx = Mutex::new();
    let counter = AtomicU32::new(0);
    let triggered_thread = AtomicU32::new(0);

    std::thread::scope(|s| {
        for _ in 0..NUMBER_OF_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                mtx.lock(|_, _| true);
                assert_that!(sut.wait(
                    &mtx,
                    |_| {},
                    |_, _| {
                        while triggered_thread.load(Ordering::Relaxed) < 1 {
                            spin_loop()
                        }
                        true
                    },
                    |_, _| true
                ), eq true);
                counter.fetch_add(1, Ordering::Relaxed);
            });
        }

        barrier.wait(|_, _| {}, |_| {});
        std::thread::sleep(TIMEOUT);
        assert_that!(counter.load(Ordering::Relaxed), eq 0);

        for i in 0..NUMBER_OF_THREADS {
            sut.notify(|_| {
                triggered_thread.fetch_add(1, Ordering::Relaxed);
            });
            std::thread::sleep(TIMEOUT);
            assert_that!(counter.load(Ordering::Relaxed), eq i + 1);
            // unlock in a different thread
            mtx.unlock(|_| {});
        }
    });
}

#[test]
fn condition_variable_wait_returns_false_when_functor_returns_false() {
    let sut = ConditionVariable::new();
    let mtx = Mutex::new();
    mtx.lock(|_, _| true);
    assert_that!(!sut.wait(&mtx, |_| {}, |_, _| false, |_, _| true), eq true);
    mtx.unlock(|_| {});
}
