//! A **threadsafe** **lock-free** single producer single consumer queue which can store [`u64`]
//! integers or indices.
//!
//! # Example
//!
//! ```
//! use elkodon_bb_lock_free::spsc::index_queue::*;
//!
//! const QUEUE_CAPACITY: usize = 128;
//! let queue = FixedSizeIndexQueue::<QUEUE_CAPACITY>::new();
//!
//! let mut producer = match queue.acquire_producer() {
//!     None => panic!("a producer has been already acquired."),
//!     Some(p) => p,
//! };
//!
//! if !producer.push(1234) {
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
    alloc::Layout,
    cell::UnsafeCell,
    fmt::Debug,
    sync::atomic::{AtomicBool, AtomicUsize, Ordering},
};

use elkodon_bb_elementary::{
    math::align_to, owning_pointer::OwningPointer, pointer_trait::PointerTrait,
    relocatable_container::RelocatableContainer, relocatable_ptr::RelocatablePointer,
};
use elkodon_bb_log::{fail, fatal_panic};

/// The [`Producer`] of the [`IndexQueue`]/[`FixedSizeIndexQueue`] which can add values to it
/// via [`Producer::push()`].
pub struct Producer<'a, PointerType: PointerTrait<UnsafeCell<usize>> + Debug> {
    queue: &'a details::IndexQueue<PointerType>,
}

impl<'a, PointerType: PointerTrait<UnsafeCell<usize>> + Debug> Producer<'a, PointerType> {
    /// Adds a new value to the [`IndexQueue`]/[`FixedSizeIndexQueue`]. If the queue is full
    /// it returns false, otherwise true.
    pub fn push(&mut self, t: usize) -> bool {
        unsafe { self.queue.push(t) }
    }
}

impl<'a, PointerType: PointerTrait<UnsafeCell<usize>> + Debug> Drop for Producer<'a, PointerType> {
    fn drop(&mut self) {
        self.queue.has_producer.store(true, Ordering::Relaxed);
    }
}

/// The [`Consumer`] of the [`IndexQueue`]/[`FixedSizeIndexQueue`] which can acquire values from it
/// via [`Consumer::pop()`].
pub struct Consumer<'a, PointerType: PointerTrait<UnsafeCell<usize>> + Debug> {
    queue: &'a details::IndexQueue<PointerType>,
}

impl<'a, PointerType: PointerTrait<UnsafeCell<usize>> + Debug> Consumer<'a, PointerType> {
    /// Acquires a value from the [`IndexQueue`]/[`FixedSizeIndexQueue`]. If the queue is empty
    /// it returns [`None`] otherwise the value.
    pub fn pop(&mut self) -> Option<usize> {
        unsafe { self.queue.pop() }
    }
}

impl<'a, PointerType: PointerTrait<UnsafeCell<usize>> + Debug> Drop for Consumer<'a, PointerType> {
    fn drop(&mut self) {
        self.queue.has_consumer.store(true, Ordering::Relaxed);
    }
}

pub type IndexQueue = details::IndexQueue<OwningPointer<UnsafeCell<usize>>>;
pub type RelocatableIndexQueue = details::IndexQueue<RelocatablePointer<UnsafeCell<usize>>>;

pub mod details {
    use std::fmt::Debug;

    use super::*;

    /// A threadsafe lock-free index queue with a capacity which can be set up at runtime, when the
    /// queue is created.
    #[repr(C)]
    #[derive(Debug)]
    pub struct IndexQueue<PointerType: PointerTrait<UnsafeCell<usize>>> {
        data_ptr: PointerType,
        capacity: usize,
        write_position: AtomicUsize,
        read_position: AtomicUsize,
        pub(super) has_producer: AtomicBool,
        pub(super) has_consumer: AtomicBool,
        is_memory_initialized: AtomicBool,
    }

    unsafe impl<PointerType: PointerTrait<UnsafeCell<usize>>> Sync for IndexQueue<PointerType> {}
    unsafe impl<PointerType: PointerTrait<UnsafeCell<usize>>> Send for IndexQueue<PointerType> {}

    impl IndexQueue<OwningPointer<UnsafeCell<usize>>> {
        pub fn new(capacity: usize) -> Self {
            let mut data_ptr = OwningPointer::<UnsafeCell<usize>>::new_with_alloc(capacity);

            for i in 0..capacity {
                unsafe { data_ptr.as_mut_ptr().add(i).write(UnsafeCell::new(0)) };
            }

            Self {
                data_ptr,
                capacity,
                write_position: AtomicUsize::new(0),
                read_position: AtomicUsize::new(0),
                has_producer: AtomicBool::new(true),
                has_consumer: AtomicBool::new(true),
                is_memory_initialized: AtomicBool::new(true),
            }
        }
    }

    impl RelocatableContainer for IndexQueue<RelocatablePointer<UnsafeCell<usize>>> {
        unsafe fn new_uninit(capacity: usize) -> Self {
            Self {
                data_ptr: RelocatablePointer::new_uninit(),
                capacity,
                write_position: AtomicUsize::new(0),
                read_position: AtomicUsize::new(0),
                has_producer: AtomicBool::new(true),
                has_consumer: AtomicBool::new(true),
                is_memory_initialized: AtomicBool::new(false),
            }
        }

        unsafe fn init<T: elkodon_bb_elementary::allocator::BaseAllocator>(
            &self,
            allocator: &T,
        ) -> Result<(), elkodon_bb_elementary::allocator::AllocationError> {
            if self.is_memory_initialized.load(Ordering::Relaxed) {
                fatal_panic!(from self, "Memory already initialized. Initializing it twice may lead to undefined behavior.");
            }

            self.data_ptr.init(fail!(from self, when allocator
            .allocate(Layout::from_size_align_unchecked(
                    std::mem::size_of::<u64>() * self.capacity,
                    std::mem::align_of::<u64>())),
            "Failed to initialize since the allocation of the data memory failed."));

            for i in 0..self.capacity {
                (self.data_ptr.as_ptr() as *mut UnsafeCell<u64>)
                    .add(i)
                    .write(UnsafeCell::new(0));
            }

            self.is_memory_initialized.store(true, Ordering::Relaxed);
            Ok(())
        }

        unsafe fn new(capacity: usize, distance_to_data: isize) -> Self {
            Self {
                data_ptr: RelocatablePointer::new(distance_to_data),
                capacity,
                write_position: AtomicUsize::new(0),
                read_position: AtomicUsize::new(0),
                has_producer: AtomicBool::new(true),
                has_consumer: AtomicBool::new(true),
                is_memory_initialized: AtomicBool::new(true),
            }
        }

        fn memory_size(capacity: usize) -> usize {
            Self::const_memory_size(capacity)
        }
    }

    impl<PointerType: PointerTrait<UnsafeCell<usize>> + Debug> IndexQueue<PointerType> {
        fn verify_init(&self, source: &str) {
            if !self.is_memory_initialized.load(Ordering::Relaxed) {
                fatal_panic!(from self, "Undefined behavior when calling \"{}\" and the object is not initialized.", source);
            }
        }

        /// Returns the amount of memory required to create a [`IndexQueue`] with the provided
        /// capacity.
        pub const fn const_memory_size(capacity: usize) -> usize {
            std::mem::size_of::<UnsafeCell<u64>>() * capacity + std::mem::align_of::<u64>() - 1
        }

        unsafe fn at(&self, position: usize) -> *mut usize {
            (*self.data_ptr.as_ptr().add(position % self.capacity)).get()
        }

        /// Acquires the [`Producer`] of the [`IndexQueue`]. This is threadsafe and lock-free without
        /// restrictions but when another thread has already acquired the [`Producer`] it returns
        /// [`None`] since it is a single producer single consumer [`IndexQueue`].
        /// ```
        /// use elkodon_bb_lock_free::spsc::index_queue::*;
        ///
        /// const QUEUE_CAPACITY: usize = 128;
        /// let queue = FixedSizeIndexQueue::<QUEUE_CAPACITY>::new();
        ///
        /// let mut producer = match queue.acquire_producer() {
        ///     None => panic!("a producer has been already acquired."),
        ///     Some(p) => p,
        /// };
        ///
        /// if !producer.push(1234) {
        ///     println!("queue is full");
        /// }
        /// ```
        pub fn acquire_producer(&self) -> Option<Producer<'_, PointerType>> {
            self.verify_init("acquire_producer");
            match self.has_producer.compare_exchange(
                true,
                false,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => Some(Producer { queue: self }),
                Err(_) => None,
            }
        }

        /// Acquires the [`Consumer`] of the [`IndexQueue`]. This is threadsafe and lock-free without
        /// restrictions but when another thread has already acquired the [`Consumer`] it returns
        /// [`None`] since it is a single producer single consumer [`IndexQueue`].
        /// ```
        /// use elkodon_bb_lock_free::spsc::index_queue::*;
        ///
        /// const QUEUE_CAPACITY: usize = 128;
        /// let queue = FixedSizeIndexQueue::<QUEUE_CAPACITY>::new();
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
        pub fn acquire_consumer(&self) -> Option<Consumer<'_, PointerType>> {
            self.verify_init("acquire_consumer");
            match self.has_consumer.compare_exchange(
                true,
                false,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => Some(Consumer { queue: self }),
                Err(_) => None,
            }
        }

        /// Pushes a value into the queue.
        ///
        /// # Safety
        ///
        ///   * Ensure that no concurrent push occurres. Only one thread at a time is allowed to call
        ///     push.
        pub unsafe fn push(&self, value: usize) -> bool {
            let write_position = self.write_position.load(Ordering::Relaxed);
            let is_full =
                write_position == self.read_position.load(Ordering::Relaxed) + self.capacity;

            if is_full {
                return false;
            }

            unsafe { self.at(write_position).write(value) };
            ////////////////
            // SYNC POINT
            ////////////////
            self.write_position
                .store(write_position + 1, Ordering::Release);

            true
        }

        /// Acquires a value from the queue.
        ///
        /// # Safety
        ///
        ///   * Ensure that no concurrent pop occurres. Only one thread at a time is allowed to call pop.
        pub unsafe fn pop(&self) -> Option<usize> {
            let read_position = self.read_position.load(Ordering::Relaxed);
            ////////////////
            // SYNC POINT
            ////////////////
            let is_empty = read_position == self.write_position.load(Ordering::Acquire);

            if is_empty {
                return None;
            }

            let value = unsafe { *self.at(read_position) };
            self.read_position
                .store(read_position + 1, Ordering::Relaxed);

            Some(value)
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

        /// Returns true when the [`IndexQueue`] is empty, otherwise false.
        /// Note: This method may make only sense in a non-concurrent setup since the information
        ///       could be out-of-date as soon as it is acquired.
        pub fn is_empty(&self) -> bool {
            let (write_position, read_position) = self.acquire_read_and_write_position();
            write_position == read_position
        }

        /// Returns the length of the [`IndexQueue`].
        /// Note: This method may make only sense in a non-concurrent setup since the information
        ///       could be out-of-date as soon as it is acquired.
        pub fn len(&self) -> usize {
            let (write_position, read_position) = self.acquire_read_and_write_position();
            write_position - read_position
        }

        /// Returns the capacity of the [`IndexQueue`].
        pub const fn capacity(&self) -> usize {
            self.capacity
        }

        /// Returns true when the [`IndexQueue`] is full, otherwise false.
        /// Note: This method may make only sense in a non-concurrent setup since the information
        ///       could be out-of-date as soon as it is acquired.
        pub fn is_full(&self) -> bool {
            let (write_position, read_position) = self.acquire_read_and_write_position();
            write_position == read_position + self.capacity
        }
    }
}

/// The compile-time fixed size version of the [`IndexQueue`].
#[derive(Debug)]
#[repr(C)]
pub struct FixedSizeIndexQueue<const CAPACITY: usize> {
    state: RelocatableIndexQueue,
    data: [UnsafeCell<u64>; CAPACITY],
}

unsafe impl<const CAPACITY: usize> Sync for FixedSizeIndexQueue<CAPACITY> {}
unsafe impl<const CAPACITY: usize> Send for FixedSizeIndexQueue<CAPACITY> {}

impl<const CAPACITY: usize> Default for FixedSizeIndexQueue<CAPACITY> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const CAPACITY: usize> FixedSizeIndexQueue<CAPACITY> {
    /// Creates a new empty [`FixedSizeIndexQueue`].
    pub fn new() -> Self {
        Self {
            state: unsafe {
                RelocatableIndexQueue::new(
                    CAPACITY,
                    align_to::<UnsafeCell<u64>>(std::mem::size_of::<RelocatableIndexQueue>())
                        as isize,
                )
            },
            data: core::array::from_fn(|_| UnsafeCell::new(0)),
        }
    }

    /// See [`IndexQueue::acquire_producer()`]
    pub fn acquire_producer(&self) -> Option<Producer<'_, RelocatablePointer<UnsafeCell<usize>>>> {
        self.state.acquire_producer()
    }

    /// See [`IndexQueue::acquire_consumer()`]
    pub fn acquire_consumer(&self) -> Option<Consumer<'_, RelocatablePointer<UnsafeCell<usize>>>> {
        self.state.acquire_consumer()
    }

    /// See [`IndexQueue::is_empty()`]
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// See [`IndexQueue::len()`]
    pub fn len(&self) -> usize {
        self.state.len()
    }

    /// See [`IndexQueue::capacity()`]
    pub const fn capacity(&self) -> usize {
        self.state.capacity()
    }

    /// See [`IndexQueue::is_full()`]
    pub fn is_full(&self) -> bool {
        self.state.is_full()
    }

    /// Pushes a value into the queue.
    ///
    /// # Safety
    ///
    ///   * Ensure that no concurrent push occurres. Only one thread at a time is allowed to call
    ///     push.
    pub unsafe fn push(&self, value: usize) -> bool {
        self.state.push(value)
    }

    /// Acquires a value from the queue.
    ///
    /// # Safety
    ///
    ///   * Ensure that no concurrent pop occurres. Only one thread at a time is allowed to call pop.
    pub unsafe fn pop(&self) -> Option<usize> {
        self.state.pop()
    }
}
