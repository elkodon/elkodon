//! Library of lock-free constructs.
//!
//! From C++ Concurrency in Action - Anthony Williams
//!
//! Obstruction-Free: If all other threads are paused, then any given thread will complete its
//!                     operation in a bounded number of steps.
//! Lock-Free: If multiple threads are operating on a data structure, then after a bounded number
//!             of steps one of them will complete its operation.
//! Wait-Free: Every thread operating on a data structure will complete its operation in a bounded
//!             number of steps, even if other threads are also operating on the data structure.
//!
//! Lock-Free guarantees that a misbehaving thread cannot block any other thread.

pub mod mpmc;
pub mod spmc;
pub mod spsc;
