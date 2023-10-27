use elkodon_bb_log::fail;
use elkodon_bb_system_types::file_name::FileName;
use std::{alloc::Layout, fmt::Debug};

use crate::shared_memory::ShmPointer;
use crate::shared_memory_directory::SharedMemoryDirectoryCreateFileError;

use super::file_reference_set::{FileReferenceSet, FileReferenceSetId};

pub struct File<'a> {
    pub(crate) set: &'a FileReferenceSet,
    pub(crate) id: FileReferenceSetId,
    pub(crate) base_address: usize,
}

impl Debug for File<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "File {{ name: {}, id: {:?}, base_address: {} }}",
            self.name(),
            self.id,
            self.base_address
        )
    }
}

impl<'a> File<'a> {
    pub fn name(&self) -> FileName {
        self.set.get_name(self.id)
    }

    pub fn content(&self) -> &[u8] {
        self.set.get_payload(self.id, self.base_address)
    }

    pub fn content_mut(&mut self) -> &mut [u8] {
        self.set.get_payload_mut(self.id, self.base_address)
    }

    pub fn is_persistent(&self) -> bool {
        self.set.is_persistent(self.id)
    }
}

impl<'a> Drop for File<'a> {
    fn drop(&mut self) {
        self.set.release(self.id)
    }
}

#[derive(Debug)]
pub struct FileCreator<'a> {
    set: &'a FileReferenceSet,
    layout: Layout,
    is_persistent: bool,
    memory: ShmPointer,
    base_address: usize,
}

impl<'a> FileCreator<'a> {
    pub(crate) fn new(
        set: &'a FileReferenceSet,
        memory: ShmPointer,
        layout: Layout,
        base_address: usize,
    ) -> Self {
        Self {
            set,
            layout,
            is_persistent: false,
            memory,
            base_address,
        }
    }

    pub fn is_persistent(mut self, value: bool) -> Self {
        self.is_persistent = value;
        self
    }

    pub fn create<F: FnMut(&mut [u8])>(
        self,
        name: &FileName,
        mut initializer: F,
    ) -> Result<File<'a>, SharedMemoryDirectoryCreateFileError> {
        let id = fail!(from self, when self.set.insert(
                                        name,
                                        self.memory.offset.value(),
                                        self.layout.size(),
                                        self.is_persistent,
                                    ),
                            "Failed to create new file {}.", *name);

        initializer(unsafe {
            core::slice::from_raw_parts_mut(self.memory.data_ptr, self.layout.size())
        });

        self.set.finalize_initialization(id);

        Ok(File {
            set: self.set,
            id,
            base_address: self.base_address,
        })
    }
}
