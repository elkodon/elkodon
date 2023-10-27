use std::{
    sync::atomic::{AtomicU32, Ordering},
    time::Duration,
};

use elkodon_bb_testing::assert_that;
use elkodon_pal_concurrency_primitives::{barrier::Barrier, rwlock::*};

const TIMEOUT: Duration = Duration::from_millis(25);

#[test]
fn rwlock_reader_preference_try_write_lock_blocks_read_locks() {
    let sut = RwLockReaderPreference::new();

    assert_that!(sut.try_write_lock(), eq true);
    assert_that!(!sut.try_write_lock(), eq true);
    assert_that!(!sut.write_lock(|_, _| false), eq true);

    assert_that!(!sut.try_read_lock(), eq true);
    assert_that!(!sut.read_lock(|_, _| false), eq true);
}

#[test]
fn rwlock_reader_preference_multiple_read_locks_block_write_lock() {
    let sut = RwLockReaderPreference::new();

    assert_that!(sut.try_read_lock(), eq true);
    assert_that!(sut.try_read_lock(), eq true);
    assert_that!(sut.read_lock(|_, _| false), eq true);
    assert_that!(sut.read_lock(|_, _| false), eq true);

    assert_that!(!sut.try_write_lock(), eq true);
    assert_that!(!sut.write_lock(|_, _| false), eq true);
}

#[test]
fn rwlock_reader_preference_write_lock_and_unlock_works() {
    let sut = RwLockReaderPreference::new();

    assert_that!(sut.write_lock(|_, _| false), eq true);

    assert_that!(!sut.try_write_lock(), eq true);
    assert_that!(!sut.try_read_lock(), eq true);
    assert_that!(!sut.read_lock(|_, _| false), eq true);

    sut.unlock(|_| {});

    assert_that!(sut.try_write_lock(), eq true);

    assert_that!(!sut.write_lock(|_, _| false), eq true);
    assert_that!(!sut.try_read_lock(), eq true);
    assert_that!(!sut.read_lock(|_, _| false), eq true);

    sut.unlock(|_| {});

    assert_that!(sut.write_lock(|_, _| false), eq true);
}

#[test]
fn rwlock_reader_preference_try_read_lock_and_unlock_works() {
    const NUMBER_OF_READ_LOCKS: usize = 123;
    let sut = RwLockReaderPreference::new();

    for _ in 0..NUMBER_OF_READ_LOCKS {
        assert_that!(sut.try_read_lock(), eq true);
        assert_that!(!sut.try_write_lock(), eq true);
        assert_that!(!sut.write_lock(|_, _| false), eq true);
    }

    for _ in 0..NUMBER_OF_READ_LOCKS {
        sut.unlock(|_| {});
    }

    assert_that!(sut.try_write_lock(), eq true);
}

#[test]
fn rwlock_reader_preference_read_lock_and_unlock_works() {
    const NUMBER_OF_READ_LOCKS: usize = 67;
    let sut = RwLockReaderPreference::new();

    for _ in 0..NUMBER_OF_READ_LOCKS {
        assert_that!(sut.read_lock(|_, _| false), eq true);
        assert_that!(!sut.try_write_lock(), eq true);
        assert_that!(!sut.write_lock(|_, _| false), eq true);
    }

    for _ in 0..NUMBER_OF_READ_LOCKS {
        sut.unlock(|_| {});
    }

    assert_that!(sut.write_lock(|_, _| false), eq true);
}

#[test]
fn rwlock_reader_preference_read_lock_blocks_only_write_locks() {
    const READ_THREADS: u32 = 4;
    const WRITE_THREADS: u32 = 4;

    let sut = RwLockReaderPreference::new();
    let barrier = Barrier::new(READ_THREADS + WRITE_THREADS + 1);
    let barrier_read = Barrier::new(READ_THREADS + 1);
    let barrier_write = Barrier::new(WRITE_THREADS + 1);

    let read_counter = AtomicU32::new(0);
    let write_counter = AtomicU32::new(0);

    std::thread::scope(|s| {
        assert_that!(sut.try_read_lock(), eq true);
        for _ in 0..WRITE_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                sut.write_lock(|_, _| true);
                write_counter.fetch_add(1, Ordering::Relaxed);
                sut.unlock(|_| {});
                barrier_write.wait(|_, _| {}, |_| {});
            });
        }

        for _ in 0..READ_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                sut.read_lock(|_, _| true);
                read_counter.fetch_add(1, Ordering::Relaxed);
                barrier_read.wait(|_, _| {}, |_| {});
                sut.unlock(|_| {});
            });
        }

        assert_that!(read_counter.load(Ordering::Relaxed), eq 0);
        assert_that!(write_counter.load(Ordering::Relaxed), eq 0);
        barrier.wait(|_, _| {}, |_| {});

        barrier_read.wait(|_, _| {}, |_| {});
        assert_that!(read_counter.load(Ordering::Relaxed), eq READ_THREADS);
        assert_that!(write_counter.load(Ordering::Relaxed), eq 0);

        sut.unlock(|_| {});
        barrier_write.wait(|_, _| {}, |_| {});
        assert_that!(write_counter.load(Ordering::Relaxed), eq WRITE_THREADS);
    });
}

#[test]
fn rwlock_reader_preference_write_lock_blocks_everything() {
    const READ_THREADS: u32 = 4;
    const WRITE_THREADS: u32 = 4;

    let sut = RwLockReaderPreference::new();
    let barrier = Barrier::new(READ_THREADS + WRITE_THREADS + 1);
    let barrier_end = Barrier::new(READ_THREADS + WRITE_THREADS + 1);

    let read_counter = AtomicU32::new(0);
    let write_counter = AtomicU32::new(0);

    std::thread::scope(|s| {
        assert_that!(sut.try_write_lock(), eq true);
        for _ in 0..WRITE_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                sut.write_lock(|_, _| true);
                let current_read_counter = read_counter.load(Ordering::Relaxed);
                write_counter.fetch_add(1, Ordering::Relaxed);
                std::thread::sleep(TIMEOUT);
                assert_that!(current_read_counter, eq read_counter.load(Ordering::Relaxed));
                sut.unlock(|_| {});

                barrier_end.wait(|_, _| {}, |_| {});
            });
        }

        for _ in 0..READ_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                sut.read_lock(|_, _| true);
                read_counter.fetch_add(1, Ordering::Relaxed);
                sut.unlock(|_| {});

                barrier_end.wait(|_, _| {}, |_| {});
            });
        }

        assert_that!(read_counter.load(Ordering::Relaxed), eq 0);
        assert_that!(write_counter.load(Ordering::Relaxed), eq 0);
        barrier.wait(|_, _| {}, |_| {});

        std::thread::sleep(TIMEOUT);
        assert_that!(read_counter.load(Ordering::Relaxed), eq 0);
        assert_that!(write_counter.load(Ordering::Relaxed), eq 0);

        sut.unlock(|_| {});

        barrier_end.wait(|_, _| {}, |_| {});
        assert_that!(read_counter.load(Ordering::Relaxed), eq READ_THREADS);
        assert_that!(write_counter.load(Ordering::Relaxed), eq WRITE_THREADS);
    });
}

//////////////////////
/// Writer Preference
//////////////////////

#[test]
fn rwlock_writer_preference_try_write_lock_blocks_read_locks() {
    let sut = RwLockWriterPreference::new();

    assert_that!(sut.try_write_lock(), eq true);
    assert_that!(!sut.try_write_lock(), eq true);
    assert_that!(!sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);

    assert_that!(!sut.try_read_lock(), eq true);
    assert_that!(!sut.read_lock(|_, _| false), eq true);
}

#[test]
fn rwlock_writer_preference_multiple_read_locks_block_write_lock() {
    let sut = RwLockWriterPreference::new();

    assert_that!(sut.try_read_lock(), eq true);
    assert_that!(sut.try_read_lock(), eq true);
    assert_that!(sut.read_lock(|_, _| false), eq true);
    assert_that!(sut.read_lock(|_, _| false), eq true);

    assert_that!(!sut.try_write_lock(), eq true);
    assert_that!(!sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);
}

#[test]
fn rwlock_writer_preference_write_lock_and_unlock_works() {
    let sut = RwLockWriterPreference::new();

    assert_that!(sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);

    assert_that!(!sut.try_write_lock(), eq true);
    assert_that!(!sut.try_read_lock(), eq true);
    assert_that!(!sut.read_lock(|_, _| false), eq true);

    sut.unlock(|_| {}, |_| {});

    assert_that!(sut.try_write_lock(), eq true);

    assert_that!(!sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);
    assert_that!(!sut.try_read_lock(), eq true);
    assert_that!(!sut.read_lock(|_, _| false), eq true);

    sut.unlock(|_| {}, |_| {});

    assert_that!(sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);
}

#[test]
fn rwlock_writer_preference_try_read_lock_and_unlock_works() {
    const NUMBER_OF_READ_LOCKS: usize = 123;
    let sut = RwLockWriterPreference::new();

    for _ in 0..NUMBER_OF_READ_LOCKS {
        assert_that!(sut.try_read_lock(), eq true);
        assert_that!(!sut.try_write_lock(), eq true);
        assert_that!(!sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);
    }

    for _ in 0..NUMBER_OF_READ_LOCKS {
        sut.unlock(|_| {}, |_| {});
    }

    assert_that!(sut.try_write_lock(), eq true);
}

#[test]
fn rwlock_writer_preference_read_lock_and_unlock_works() {
    const NUMBER_OF_READ_LOCKS: usize = 67;
    let sut = RwLockWriterPreference::new();

    for _ in 0..NUMBER_OF_READ_LOCKS {
        assert_that!(sut.read_lock(|_, _| false), eq true);
        assert_that!(!sut.try_write_lock(), eq true);
        assert_that!(!sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);
    }

    for _ in 0..NUMBER_OF_READ_LOCKS {
        sut.unlock(|_| {}, |_| {});
    }

    assert_that!(sut.write_lock(|_, _| false, |_| {}, |_| {}), eq true);
}

#[test]
fn rwlock_writer_preference_write_lock_blocks_everything() {
    const READ_THREADS: u32 = 4;
    const WRITE_THREADS: u32 = 4;

    let sut = RwLockWriterPreference::new();
    let barrier = Barrier::new(READ_THREADS + WRITE_THREADS + 1);
    let barrier_end = Barrier::new(READ_THREADS + WRITE_THREADS + 1);

    let read_counter = AtomicU32::new(0);
    let write_counter = AtomicU32::new(0);

    std::thread::scope(|s| {
        assert_that!(sut.try_write_lock(), eq true);
        for _ in 0..WRITE_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                sut.write_lock(|_, _| true, |_| {}, |_| {});
                let current_read_counter = read_counter.load(Ordering::Relaxed);
                write_counter.fetch_add(1, Ordering::Relaxed);
                std::thread::sleep(TIMEOUT);
                assert_that!(current_read_counter, eq read_counter.load(Ordering::Relaxed));
                sut.unlock(|_| {}, |_| {});

                barrier_end.wait(|_, _| {}, |_| {});
            });
        }

        for _ in 0..READ_THREADS {
            s.spawn(|| {
                barrier.wait(|_, _| {}, |_| {});
                sut.read_lock(|_, _| true);
                read_counter.fetch_add(1, Ordering::Relaxed);
                sut.unlock(|_| {}, |_| {});

                barrier_end.wait(|_, _| {}, |_| {});
            });
        }

        assert_that!(read_counter.load(Ordering::Relaxed), eq 0);
        assert_that!(write_counter.load(Ordering::Relaxed), eq 0);
        barrier.wait(|_, _| {}, |_| {});

        std::thread::sleep(TIMEOUT);
        assert_that!(read_counter.load(Ordering::Relaxed), eq 0);
        assert_that!(write_counter.load(Ordering::Relaxed), eq 0);

        sut.unlock(|_| {}, |_| {});

        barrier_end.wait(|_, _| {}, |_| {});
        assert_that!(read_counter.load(Ordering::Relaxed), eq READ_THREADS);
        assert_that!(write_counter.load(Ordering::Relaxed), eq WRITE_THREADS);
    });
}
