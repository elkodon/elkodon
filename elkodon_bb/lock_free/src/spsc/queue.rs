//! A **threadsafe** **lock-free** single produce single consumer queue.
//! **IMPORTANT** Can only be used with trivially copyable types which are also trivially dropable.
//!
//! # Example
//!
//! ```
//! use elkodon_bb_lock_free::spsc::queue::*;
//!
//! const QUEUE_CAPACITY: usize = 128;
//! let queue = Queue::<u64, QUEUE_CAPACITY>::new();
//!
//! let mut producer = match queue.acquire_producer() {
//!     None => panic!("a producer has been already acquired."),
//!     Some(p) => p,
//! };
//!
//! if !producer.push(&1234) {
//!     println!("queue is full");
//! }
//!
//!
//! let mut consumer = match queue.acquire_consumer() {
//!     None => panic!("a consumer has been already acquired."),
//!     Some(p) => p,
//! };
//!
//! match consumer.pop() {
//!     None => println!("queue is empty"),
//!     Some(v) => println!("got {}", v)
//! }
//! ```

use std::{
    cell::UnsafeCell,
    mem::MaybeUninit,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

/// The [`Producer`] of the [`Queue`] which can add values to it via [`Producer::push()`].
pub struct Producer<'a, T: Copy, const CAPACITY: usize> {
    queue: &'a Queue<T, CAPACITY>,
}

impl<T: Copy, const CAPACITY: usize> Producer<'_, T, CAPACITY> {
    /// Adds a new value to the queue, if the queue is full it returns false otherwise true
    pub fn push(&mut self, t: &T) -> bool {
        unsafe { self.queue.push(t) }
    }
}

impl<T: Copy, const CAPACITY: usize> Drop for Producer<'_, T, CAPACITY> {
    fn drop(&mut self) {
        self.queue.has_producer.store(true, Ordering::Relaxed);
    }
}

/// The [`Consumer`] of the [`Queue`] which can acquire values from it via [`Consumer::pop()`].
pub struct Consumer<'a, T: Copy, const CAPACITY: usize> {
    queue: &'a Queue<T, CAPACITY>,
}

impl<T: Copy, const CAPACITY: usize> Consumer<'_, T, CAPACITY> {
    /// Removes the oldest element from the queue. If the queue is empty it returns [`None`]
    pub fn pop(&mut self) -> Option<T> {
        unsafe { self.queue.pop() }
    }
}

impl<T: Copy, const CAPACITY: usize> Drop for Consumer<'_, T, CAPACITY> {
    fn drop(&mut self) {
        self.queue.has_consumer.store(true, Ordering::Relaxed);
    }
}

/// The threadsafe lock-free with a compile time fixed capacity.
pub struct Queue<T: Copy, const CAPACITY: usize> {
    data: [UnsafeCell<MaybeUninit<T>>; CAPACITY],
    write_position: AtomicUsize,
    read_position: AtomicUsize,
    has_producer: AtomicBool,
    has_consumer: AtomicBool,
}

unsafe impl<T: Copy + Sync, const CAPACITY: usize> Sync for Queue<T, CAPACITY> {}

impl<T: Copy, const CAPACITY: usize> Queue<T, CAPACITY> {
    /// Creates a new empty queue
    pub fn new() -> Self {
        Self {
            data: core::array::from_fn(|_| UnsafeCell::new(MaybeUninit::uninit())),
            write_position: AtomicUsize::new(0),
            read_position: AtomicUsize::new(0),
            has_producer: AtomicBool::new(true),
            has_consumer: AtomicBool::new(true),
        }
    }

    /// Returns a [`Producer`] to add data to the queue. If a producer was already
    /// acquired it returns [`None`].
    /// ```
    /// use elkodon_bb_lock_free::spsc::queue::*;
    ///
    /// const QUEUE_CAPACITY: usize = 128;
    /// let queue = Queue::<u64, QUEUE_CAPACITY>::new();
    ///
    /// let mut producer = match queue.acquire_producer() {
    ///     None => panic!("a producer has been already acquired."),
    ///     Some(p) => p,
    /// };
    ///
    /// if !producer.push(&1234) {
    ///     println!("queue is full");
    /// }
    /// ```
    pub fn acquire_producer(&self) -> Option<Producer<'_, T, CAPACITY>> {
        match self
            .has_producer
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => Some(Producer { queue: self }),
            Err(_) => None,
        }
    }

    /// Returns a [`Consumer`] to acquire data from the queue. If a consumer was already
    /// acquired it returns [`None`].
    /// ```
    /// use elkodon_bb_lock_free::spsc::queue::*;
    ///
    /// const QUEUE_CAPACITY: usize = 128;
    /// let queue = Queue::<u64, QUEUE_CAPACITY>::new();
    ///
    /// let mut consumer = match queue.acquire_consumer() {
    ///     None => panic!("a consumer has been already acquired."),
    ///     Some(p) => p,
    /// };
    ///
    /// match consumer.pop() {
    ///     None => println!("queue is empty"),
    ///     Some(v) => println!("got {}", v)
    /// }
    /// ```
    pub fn acquire_consumer(&self) -> Option<Consumer<'_, T, CAPACITY>> {
        match self
            .has_consumer
            .compare_exchange(true, false, Ordering::Relaxed, Ordering::Relaxed)
        {
            Ok(_) => Some(Consumer { queue: self }),
            Err(_) => None,
        }
    }
    /// Push an index into the [`Queue`]. If the queue is full the oldest
    /// index is returned and replaced with the new value.
    ///
    /// # Safety
    ///
    ///  * [`Queue::push()`] cannot be called concurrently. The user has
    ///    to ensure that at most one thread access this method.
    pub unsafe fn push(&self, t: &T) -> bool {
        let current_write_pos = self.write_position.load(Ordering::Relaxed);
        let is_full = current_write_pos == self.read_position.load(Ordering::Relaxed) + CAPACITY;

        match is_full {
            true => false,
            false => {
                unsafe {
                    self.data[current_write_pos % CAPACITY]
                        .get()
                        .write(MaybeUninit::new(*t));
                }
                ////////////////
                // SYNC POINT
                ////////////////
                self.write_position
                    .store(current_write_pos + 1, Ordering::Release);
                true
            }
        }
    }

    /// Acquires an index from the [`Queue`]. If the queue is empty
    /// [`None`] is returned.
    ///
    /// # Safety
    ///
    ///  * [`Queue::pop()`] cannot be called concurrently. The user has
    ///    to ensure that at most one thread access this method.
    pub unsafe fn pop(&self) -> Option<T> {
        let current_read_pos = self.read_position.load(Ordering::Relaxed);
        ////////////////
        // SYNC POINT
        ////////////////
        let is_empty = current_read_pos == self.write_position.load(Ordering::Acquire);

        match is_empty {
            true => None,
            false => {
                let out: T = unsafe {
                    *self.data[current_read_pos % CAPACITY]
                        .get()
                        .as_ref()
                        .unwrap()
                        .as_ptr()
                };

                self.read_position
                    .store(current_read_pos + 1, Ordering::Release);

                Some(out)
            }
        }
    }

    fn acquire_read_and_write_position(&self) -> (usize, usize) {
        loop {
            let write_position = self.write_position.load(Ordering::Relaxed);
            let read_position = self.read_position.load(Ordering::Relaxed);

            if write_position == self.write_position.load(Ordering::Relaxed)
                && read_position == self.read_position.load(Ordering::Relaxed)
            {
                return (write_position, read_position);
            }
        }
    }

    /// Returns true if the queue is empty, otherwise false
    pub fn is_empty(&self) -> bool {
        let (write_position, read_position) = self.acquire_read_and_write_position();
        write_position == read_position
    }

    /// Returns the number of elements stored in the queue
    pub fn len(&self) -> usize {
        let (write_position, read_position) = self.acquire_read_and_write_position();
        write_position - read_position
    }

    /// Returns the overall capacity of the queue
    pub fn capacity(&self) -> usize {
        CAPACITY
    }

    /// Returns true if the queue is full, otherwise false
    pub fn is_full(&self) -> bool {
        let (write_position, read_position) = self.acquire_read_and_write_position();
        write_position == read_position + CAPACITY
    }
}

impl<T: Copy, const CAPACITY: usize> Default for Queue<T, CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}
