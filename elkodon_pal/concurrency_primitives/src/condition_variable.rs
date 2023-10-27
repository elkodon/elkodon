use core::sync::atomic::{AtomicU32, Ordering};

pub use crate::mutex::Mutex;

pub struct ConditionVariable {
    counter: AtomicU32,
}

impl Default for ConditionVariable {
    fn default() -> Self {
        Self {
            counter: AtomicU32::new(0),
        }
    }
}

impl ConditionVariable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn notify<WakeOneOrAll: Fn(&AtomicU32)>(&self, wake_one_or_all: WakeOneOrAll) {
        self.counter.fetch_add(1, Ordering::Relaxed);
        wake_one_or_all(&self.counter);
    }

    pub fn wait<
        WakeOne: Fn(&AtomicU32),
        Wait: Fn(&AtomicU32, &u32) -> bool,
        MtxWait: Fn(&AtomicU32, &u32) -> bool,
    >(
        &self,
        mtx: &Mutex,
        mtx_wake_one: WakeOne,
        wait: Wait,
        mtx_wait: MtxWait,
    ) -> bool {
        let counter_value = self.counter.load(Ordering::Relaxed);
        mtx.unlock(mtx_wake_one);

        if !wait(&self.counter, &counter_value) {
            return false;
        }

        // this maybe problematic when the wait has a timeout. it is possible that
        // the timeout is nearly doubled when wait is waken up at the end of the timeout
        // as well as the mtx lock
        mtx.lock(mtx_wait)
    }
}
