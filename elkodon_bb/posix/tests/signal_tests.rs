// Copyright (c) 2023 Contributors to the Eclipse Foundation
//
// See the NOTICE file(s) distributed with this work for additional
// information regarding copyright ownership.
//
// This program and the accompanying materials are made available under the
// terms of the Apache Software License 2.0 which is available at
// https://www.apache.org/licenses/LICENSE-2.0, or the MIT license
// which is available at https://opensource.org/licenses/MIT.
//
// SPDX-License-Identifier: Apache-2.0 OR MIT

use elkodon_bb_posix::clock::*;
use elkodon_bb_posix::process::*;
use elkodon_bb_posix::signal::*;
use elkodon_bb_testing::assert_that;
use elkodon_bb_testing::test_requires;
use elkodon_pal_posix::posix::POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING;
use elkodon_pal_posix::*;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::thread;
use std::time::Duration;

static COUNTER: AtomicUsize = AtomicUsize::new(0);
static SIGNAL: AtomicUsize = AtomicUsize::new(posix::MAX_SIGNAL_VALUE);
static LOCK: Mutex<i32> = Mutex::new(0);
const TIMEOUT: Duration = Duration::from_millis(100);

struct TestFixture {
    _guard: MutexGuard<'static, i32>,
}

impl TestFixture {
    fn new() -> Self {
        let new_self = Self {
            _guard: LOCK.lock().unwrap(),
        };

        COUNTER.store(0, Ordering::SeqCst);
        SIGNAL.store(posix::MAX_SIGNAL_VALUE, Ordering::SeqCst);

        new_self
    }

    pub fn signal_callback(signal: FetchableSignal) {
        SIGNAL.store(signal as usize, Ordering::SeqCst);
        COUNTER.fetch_add(1, Ordering::SeqCst);
    }

    pub fn verify(&self, signal: FetchableSignal, counter: usize) {
        assert_that!(
            || { COUNTER.load(Ordering::SeqCst) == counter },
            block_until_true
        );

        assert_that!(SignalHandler::last_signal(), eq Some(signal));
        assert_that!(SIGNAL.load(Ordering::SeqCst), eq signal as usize);
    }
}

#[test]
fn signal_register_single_handler_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let test = TestFixture::new();
    let _guard =
        SignalHandler::register(FetchableSignal::UserDefined1, &TestFixture::signal_callback);

    Process::from_self().send_signal(Signal::UserDefined1).ok();
    test.verify(FetchableSignal::UserDefined1, 1)
}

#[test]
fn signal_register_multiple_handler_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let test = TestFixture::new();
    let _guard1 =
        SignalHandler::register(FetchableSignal::UserDefined1, &TestFixture::signal_callback);

    let _guard2 =
        SignalHandler::register(FetchableSignal::UserDefined2, &TestFixture::signal_callback);

    Process::from_self().send_signal(Signal::UserDefined1).ok();
    test.verify(FetchableSignal::UserDefined1, 1);

    Process::from_self().send_signal(Signal::UserDefined2).ok();
    test.verify(FetchableSignal::UserDefined2, 2);
}

#[test]
fn signal_register_handler_with_multiple_signals_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let test = TestFixture::new();
    let s = vec![FetchableSignal::UserDefined1, FetchableSignal::UserDefined2];
    let _guard1 = SignalHandler::register_multiple_signals(&s, &TestFixture::signal_callback);

    Process::from_self().send_signal(Signal::UserDefined1).ok();
    test.verify(FetchableSignal::UserDefined1, 1);

    Process::from_self().send_signal(Signal::UserDefined2).ok();
    test.verify(FetchableSignal::UserDefined2, 2);
}

#[test]
fn signal_guard_unregisters_on_drop() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let test = TestFixture::new();
    let guard1 =
        SignalHandler::register(FetchableSignal::UserDefined1, &TestFixture::signal_callback);

    drop(guard1);

    let _guard1 = SignalHandler::register(FetchableSignal::UserDefined1, &|signal| {
        SIGNAL.store(signal as usize, Ordering::SeqCst);
        COUNTER.fetch_add(10, Ordering::SeqCst);
    });

    Process::from_self().send_signal(Signal::UserDefined1).ok();
    test.verify(FetchableSignal::UserDefined1, 10);
}

#[test]
fn signal_register_signal_twice_fails() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();
    let s = vec![FetchableSignal::UserDefined1, FetchableSignal::UserDefined2];
    let _guard1 = SignalHandler::register_multiple_signals(&s, &TestFixture::signal_callback);

    assert_that!(
        SignalHandler::register(FetchableSignal::UserDefined2, &TestFixture::signal_callback),
        is_err
    );
}

#[test]
fn signal_call_and_fetch_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    let result = SignalHandler::call_and_fetch(|| {
        Process::from_self().send_signal(Signal::Interrupt).ok();
        nanosleep(TIMEOUT).ok();
    });

    assert_that!(result, eq Some(FetchableSignal::Interrupt));
}

#[test]
fn signal_call_and_fetch_with_registered_handler_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let test = TestFixture::new();

    let _guard =
        SignalHandler::register(FetchableSignal::UserDefined1, &TestFixture::signal_callback);

    let result = SignalHandler::call_and_fetch(|| {
        Process::from_self().send_signal(Signal::UserDefined1).ok();
        nanosleep(TIMEOUT).ok();
    });

    assert_that!(result, eq Some(FetchableSignal::UserDefined1));
    test.verify(FetchableSignal::UserDefined1, 1);
}

#[test]
fn signal_wait_for_signal_blocks() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    let signals = vec![FetchableSignal::UserDefined2, FetchableSignal::UserDefined1];
    let counter = AtomicI32::new(0);
    thread::scope(|s| {
        s.spawn(|| {
            SignalHandler::wait_for_multiple_signals(&signals).unwrap();
            counter.store(1, Ordering::Relaxed);
        });

        nanosleep(TIMEOUT).ok();
        let counter_old = counter.load(Ordering::Relaxed);
        Process::from_self().send_signal(Signal::UserDefined2).ok();

        assert_that!(counter_old, eq 0);
        assert_that!(
            || { counter.load(Ordering::Relaxed) == 1 },
            block_until_true
        );
    });
}

#[test]
fn signal_wait_twice_for_same_signal_blocks() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    let counter = AtomicI32::new(0);
    thread::scope(|s| {
        s.spawn(|| {
            SignalHandler::wait_for_signal(FetchableSignal::UserDefined2).unwrap();
            counter.fetch_add(1, Ordering::Relaxed);
        });

        nanosleep(TIMEOUT).ok();
        let counter_old = counter.load(Ordering::Relaxed);
        Process::from_self().send_signal(Signal::UserDefined2).ok();

        s.spawn(|| {
            SignalHandler::wait_for_signal(FetchableSignal::UserDefined2).unwrap();
            counter.fetch_add(1, Ordering::Relaxed);
        });

        nanosleep(TIMEOUT).ok();
        let counter_old_2 = counter.load(Ordering::Relaxed);
        Process::from_self().send_signal(Signal::UserDefined2).ok();

        assert_that!(counter_old, eq 0);
        assert_that!(counter_old_2, le 1);
        assert_that!(
            || { counter.load(Ordering::Relaxed) == 2 },
            block_until_true
        );
    });
}

#[test]
fn signal_timed_wait_blocks_at_least_for_timeout() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    let start = Time::now_with_clock(ClockType::Monotonic).unwrap();
    SignalHandler::timed_wait_for_signal(FetchableSignal::UserDefined2, TIMEOUT).unwrap();
    assert_that!(start.elapsed().unwrap(), time_at_least TIMEOUT);
}

#[test]
fn signal_timed_wait_blocks_until_signal() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    let signals = vec![FetchableSignal::UserDefined2, FetchableSignal::UserDefined1];
    let counter = AtomicI32::new(0);
    thread::scope(|s| {
        s.spawn(|| {
            SignalHandler::timed_wait_for_multiple_signals(&signals, 100 * TIMEOUT).unwrap();
            counter.store(1, Ordering::Relaxed);
        });

        nanosleep(TIMEOUT).ok();
        let counter_old = counter.load(Ordering::Relaxed);
        Process::from_self().send_signal(Signal::UserDefined2).ok();

        assert_that!(counter_old, eq 0);
        assert_that!(
            || { counter.load(Ordering::Relaxed) == 1 },
            block_until_true
        );
    });
}

#[test]
fn signal_termination_requested_with_terminate_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    assert_that!(!SignalHandler::termination_requested(), eq true);
    assert_that!(Process::from_self().send_signal(Signal::Terminate), is_ok);

    assert_that!(
        || { SignalHandler::termination_requested() == true },
        block_until_true
    );
    assert_that!(SignalHandler::termination_requested(), eq false);
}

#[test]
fn signal_termination_requested_with_interrupt_works() {
    test_requires!(POSIX_SUPPORT_ADVANCED_SIGNAL_HANDLING);

    let _test = TestFixture::new();

    assert_that!(SignalHandler::termination_requested(), eq false);
    assert_that!(Process::from_self().send_signal(Signal::Interrupt), is_ok);

    assert_that!(
        || { SignalHandler::termination_requested() == true },
        block_until_true
    );
    assert_that!(SignalHandler::termination_requested(), eq false);
}
