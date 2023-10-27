use elkodon_bb_posix::adaptive_wait::*;
use elkodon_bb_posix::clock::*;
use elkodon_bb_posix::config::*;
use elkodon_bb_testing::assert_that;
use std::time::Duration;
use std::time::Instant;

const TIMEOUT: Duration = Duration::from_millis(50);

#[test]
fn adaptive_wait_wait_loop_initial_repetitions_plus_one_is_at_least_final_waiting_time() {
    let start = Instant::now();
    let mut counter: u64 = 0;

    AdaptiveWaitBuilder::new()
        .create()
        .unwrap()
        .wait_while(move || -> bool {
            counter += 1;
            counter < ADAPTIVE_WAIT_INITIAL_REPETITIONS
        })
        .expect("failed to test wait_loop");

    assert_that!(start.elapsed(), ge ADAPTIVE_WAIT_FINAL_WAITING_TIME);
}

#[test]
fn adaptive_wait_on_default_builder_uses_default_clock() {
    let sut = AdaptiveWaitBuilder::new().create().unwrap();
    assert_that!(sut.clock_type(), eq ClockType::default());
}

#[test]
fn adaptive_wait_custom_clock_is_set_correctly() {
    let sut = AdaptiveWaitBuilder::new()
        .clock_type(ClockType::Realtime)
        .create()
        .unwrap();
    assert_that!(sut.clock_type(), eq ClockType::Realtime);
}

#[test]
fn adaptive_wait_wait_increases_yield_counter() {
    let mut sut = AdaptiveWaitBuilder::new().create().unwrap();
    assert_that!(sut.wait(), is_ok);
    assert_that!(sut.wait(), is_ok);
    assert_that!(sut.wait(), is_ok);
    assert_that!(sut.yield_count(), eq 3);
}

#[test]
fn adaptive_wait_timed_wait_while_wait_at_least_for_timeout() {
    let mut sut = AdaptiveWaitBuilder::new().create().unwrap();
    let start = Instant::now();

    let result = sut
        .timed_wait_while(|| -> Result<bool, ()> { Ok(true) }, TIMEOUT)
        .unwrap();

    assert_that!(start.elapsed(), ge TIMEOUT);
    assert_that!(result, eq false);
}

#[test]
fn adaptive_wait_timed_wait_does_not_wait_when_predicate_returns_false() {
    let mut sut = AdaptiveWaitBuilder::new().create().unwrap();
    let start = Instant::now();

    let result = sut
        .timed_wait_while(|| -> Result<bool, ()> { Ok(false) }, TIMEOUT)
        .unwrap();

    assert_that!(start.elapsed(), lt TIMEOUT);
    assert_that!(result, eq true);
}

#[test]
fn adaptive_wait_timed_wait_does_not_wait_when_predicate_returns_error() {
    let mut sut = AdaptiveWaitBuilder::new().create().unwrap();
    let start = Instant::now();

    let result = sut.timed_wait_while(|| -> Result<bool, i32> { Err(5) }, TIMEOUT);

    assert_that!(start.elapsed(), lt TIMEOUT);
    assert_that!(result, is_err);
    assert_that!(
        result.err().unwrap(), eq
        AdaptiveTimedWaitWhileError::<i32>::PredicateFailure(5)
    );
}
