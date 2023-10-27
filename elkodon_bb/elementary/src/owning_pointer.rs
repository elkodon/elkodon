//! Represents a normal non-null pointer. It was introduced to distinguish normal pointers from
//! [`crate::relocatable_ptr::RelocatablePointer`]. It implements the [`PointerTrait`].

use std::alloc::Layout;
use std::alloc::{alloc, dealloc};

use crate::pointer_trait::PointerTrait;

/// Representation of a pointer which owns its memory.
#[repr(C)]
#[derive(Debug)]
pub struct OwningPointer<T> {
    ptr: *mut T,
    layout: Layout,
}

impl<T> OwningPointer<T> {
    /// Allocates memory for T and number_of_elements. If the number_of_elements is zero it still
    /// allocates memory for one element.
    pub fn new_with_alloc(mut number_of_elements: usize) -> OwningPointer<T> {
        if number_of_elements == 0 {
            number_of_elements = 1;
        }

        let layout = unsafe {
            Layout::from_size_align_unchecked(
                std::mem::size_of::<T>() * number_of_elements,
                std::mem::align_of::<T>(),
            )
        };

        Self {
            ptr: unsafe { alloc(layout) as *mut T },
            layout,
        }
    }
}

impl<T> Drop for OwningPointer<T> {
    fn drop(&mut self) {
        unsafe { dealloc(self.ptr as *mut u8, self.layout) }
    }
}

impl<T> PointerTrait<T> for OwningPointer<T> {
    unsafe fn as_ptr(&self) -> *const T {
        self.ptr as *const T
    }

    unsafe fn as_mut_ptr(&mut self) -> *mut T {
        self.ptr
    }
}
