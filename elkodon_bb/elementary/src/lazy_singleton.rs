//! Can be used to implement a singleton object which is not initialized when it is being created.
//!
//! Useful for global logger, error handling or config objects which are initialized sometime
//! during the startup phase. The object itself is not a singleton.
//!
//! # Example
//!
//! ```
//! use elkodon_bb_elementary::lazy_singleton::*;
//!
//! static LAZY_GLOBAL: LazySingleton<u64> = LazySingleton::<u64>::new();
//!
//! // in startup phase
//! if LAZY_GLOBAL.set_value(1234) {
//!     println!("successfully initialized");
//! } else {
//!     println!("someone else already initialized the object");
//! }
//!
//! // during runtime from multiple threads
//! println!("{}", LAZY_GLOBAL.get());
//! ```

use std::{
    cell::UnsafeCell,
    sync::atomic::{AtomicBool, Ordering},
};

/// The lazy initialized singleton building block of type T
#[derive(Debug)]
pub struct LazySingleton<T> {
    data: UnsafeCell<Option<T>>,
    is_initialized: AtomicBool,
    is_finalized: AtomicBool,
}

unsafe impl<T: Send> Send for LazySingleton<T> {}
unsafe impl<T: Send + Sync> Sync for LazySingleton<T> {}

impl<T> LazySingleton<T> {
    /// Creates a new [`LazySingleton`] where the underlying value is not yet initialized.
    pub const fn new() -> Self {
        Self {
            data: UnsafeCell::new(None),
            is_initialized: AtomicBool::new(false),
            is_finalized: AtomicBool::new(false),
        }
    }

    /// Returns true if the underlying value was initialized, otherwise false.
    pub fn is_initialized(&self) -> bool {
        self.is_initialized.load(Ordering::Relaxed)
    }

    /// Sets the value of the uninitialized [`LazySingleton`]. If it was already initialized it
    /// returns false, otherwise true.
    pub fn set_value(&self, value: T) -> bool {
        let is_initialized =
            self.is_initialized
                .compare_exchange(false, true, Ordering::Relaxed, Ordering::Relaxed);

        if is_initialized.is_err() {
            return false;
        }

        unsafe { *self.data.get() = Some(value) };
        self.is_finalized.store(true, Ordering::Release);
        true
    }

    /// Returns a reference to the underlying object. If the [`LazySingleton`] does not contain
    /// any object it panics.
    pub fn get(&self) -> &T {
        if self.is_finalized.load(Ordering::Acquire) {
            return unsafe { self.data.get().as_ref().unwrap().as_ref().unwrap() };
        }

        if !self.is_initialized.load(Ordering::Relaxed) {
            panic!("You cannot acquire an unset value");
        }

        while !self.is_finalized.load(Ordering::Acquire) {
            std::hint::spin_loop()
        }
        unsafe { self.data.get().as_ref().unwrap().as_ref().unwrap() }
    }
}
