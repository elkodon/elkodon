//! A **non-threadsafe** [`Allocator`] which manages only on chunk. When allocating memory always the
//! maximum amount of available aligned memory is provided.
//!
//! # Example
//! ```
//! use elkodon_bb_memory::one_chunk_allocator::*;
//!
//! const MEMORY_SIZE: usize = 1024;
//! let mut memory: [u8; MEMORY_SIZE] = [0; MEMORY_SIZE];
//! let mut allocator = OneChunkAllocator::new(NonNull::new(memory.as_mut_ptr()).unwrap(),
//!                                             MEMORY_SIZE);
//!
//! // always returns a slice with the maximum available size
//! let mut memory = allocator.allocate(unsafe{Layout::from_size_align_unchecked(48, 4)})
//!                           .expect("failed to allocate");
//!
//! // will always return the same pointer but shrink the underlying memory
//! let mut shrink_memory = unsafe { allocator.shrink(
//!                             NonNull::new(memory.as_mut().as_mut_ptr()).unwrap(),
//!                             Layout::from_size_align_unchecked(64, 4),
//!                             Layout::from_size_align_unchecked(32, 4)
//!                         ).expect("failed to shrink memory")};
//!
//! // will always return the same pointer but grow the underlying memory
//! let mut grown_memory = unsafe { allocator.grow_zeroed(
//!                             NonNull::new(shrink_memory.as_mut().as_mut_ptr()).unwrap(),
//!                             Layout::from_size_align_unchecked(48, 4),
//!                             Layout::from_size_align_unchecked(64, 4)
//!                         ).expect("failed to grow memory")};
//!
//! unsafe{ allocator.deallocate(NonNull::new(grown_memory.as_mut().as_mut_ptr()).unwrap(),
//!                              Layout::from_size_align_unchecked(32, 4))};
//! ```
use elkodon_bb_log::error;
use std::sync::atomic::{AtomicUsize, Ordering};

pub use elkodon_bb_elementary::allocator::*;
use elkodon_bb_elementary::math::align;
pub use std::alloc::Layout;

#[derive(Debug)]
pub struct OneChunkAllocator {
    start: usize,
    size: usize,
    allocated_chunk_start: AtomicUsize,
}

impl OneChunkAllocator {
    pub fn new(ptr: NonNull<u8>, size: usize) -> OneChunkAllocator {
        OneChunkAllocator {
            start: ptr.as_ptr() as usize,
            size,
            allocated_chunk_start: AtomicUsize::new(0),
        }
    }

    pub fn has_chunk_available(&self) -> bool {
        self.allocated_chunk_start.load(Ordering::Relaxed) == 0
    }

    fn is_allocated_chunk(&self, ptr: &NonNull<u8>) -> bool {
        ptr.as_ptr() as usize == self.allocated_chunk_start.load(Ordering::Relaxed)
    }

    fn release_chunk(&self) {
        self.allocated_chunk_start.store(0, Ordering::Relaxed)
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn size(&self) -> usize {
        self.size
    }
}

impl BaseAllocator for OneChunkAllocator {
    fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocationError> {
        let adjusted_start = align(self.start, layout.align());
        let msg = "Unable to allocate chunk";

        if !self.has_chunk_available() {
            error!(from self, "{} since there is no more chunk available.", msg);
            return Err(AllocationError::OutOfMemory);
        }

        let available_size = self.size - (adjusted_start - self.start);
        if available_size <= layout.size() {
            error!(from self, "{} due to insufficient available memory.", msg);
            return Err(AllocationError::OutOfMemory);
        }

        self.allocated_chunk_start
            .store(adjusted_start, Ordering::Relaxed);
        Ok(NonNull::new(unsafe {
            std::slice::from_raw_parts_mut(adjusted_start as *mut u8, available_size)
        })
        .unwrap())
    }

    unsafe fn deallocate(
        &self,
        ptr: NonNull<u8>,
        _layout: Layout,
    ) -> Result<(), DeallocationError> {
        match self.is_allocated_chunk(&ptr) {
            true => {
                self.release_chunk();
                Ok(())
            }
            false => {
                error!(from self, "Tried to release memory ({}) which does not belong to this allocator.", ptr.as_ptr() as usize);
                Err(DeallocationError::ProvidedPointerNotContainedInAllocator)
            }
        }
    }
}

impl Allocator for OneChunkAllocator {
    unsafe fn grow(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocationGrowError> {
        let msg = "Unable to grow memory chunk";
        if !self.is_allocated_chunk(&ptr) {
            error!(from self, "{} since the provided pointer is not contained in this allocator.", msg);
            return Err(AllocationGrowError::ProvidedPointerNotContainedInAllocator);
        }

        if old_layout.size() >= new_layout.size() {
            error!(from self, "{} since the new size {} is smaller than the old size {}.", msg, new_layout.size(), old_layout.size());
            return Err(AllocationGrowError::GrowWouldShrink);
        }

        if old_layout.align() < new_layout.align() {
            error!(from self, "{} since this allocator does not support to any alignment increase in this operation.", msg);
            return Err(AllocationGrowError::AlignmentFailure);
        }

        let available_size =
            self.size - (self.allocated_chunk_start.load(Ordering::Relaxed) - self.start);

        if available_size < new_layout.size() {
            error!(from self, "{} since the size of {} exceeds the available memory size of {}.", msg, new_layout.size(), available_size);
            return Err(AllocationGrowError::OutOfMemory);
        }

        Ok(NonNull::new(std::slice::from_raw_parts_mut(ptr.as_ptr(), available_size)).unwrap())
    }

    unsafe fn shrink(
        &self,
        ptr: NonNull<u8>,
        old_layout: Layout,
        new_layout: Layout,
    ) -> Result<NonNull<[u8]>, AllocationShrinkError> {
        let msg = "Unable to shrink memory chunk";
        if !self.is_allocated_chunk(&ptr) {
            error!(from self, "{} since the provided pointer is not contained in this allocator.", msg);
            return Err(AllocationShrinkError::ProvidedPointerNotContainedInAllocator);
        }

        if old_layout.size() <= new_layout.size() {
            error!(from self, "{} since the new size {} is greater than the old size {}.", msg, new_layout.size(), old_layout.size());
            return Err(AllocationShrinkError::ShrinkWouldGrow);
        }

        if old_layout.align() < new_layout.align() {
            error!(from self, "{} since this allocator does not support to any alignment increase in this operation.", msg);
            return Err(AllocationShrinkError::AlignmentFailure);
        }

        Ok(NonNull::new(std::slice::from_raw_parts_mut(
            ptr.as_ptr(),
            new_layout.size(),
        ))
        .unwrap())
    }
}
