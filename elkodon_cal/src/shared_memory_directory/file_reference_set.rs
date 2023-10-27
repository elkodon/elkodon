use crate::shared_memory_directory::decision_counter::DecisionCounter;
use crate::shared_memory_directory::file::File;
use crate::shared_memory_directory::reference_counter::ReferenceCounter;
use crate::shared_memory_directory::SharedMemoryDirectoryCreateFileError;
use crate::shared_memory_directory::MAX_NUMBER_OF_ENTRIES;
use elkodon_bb_lock_free::mpmc::unique_index_set::FixedSizeUniqueIndexSet;
use elkodon_bb_log::fail;
use elkodon_bb_system_types::file_name::FileName;
use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy)]
pub(crate) struct FileReferenceSetId(usize);

#[derive(Debug, Clone, Copy)]
#[repr(C)]
struct Entry {
    name: FileName,
    offset: usize,
    len: usize,
}

impl Entry {
    const fn default() -> Self {
        Self {
            name: unsafe { FileName::new_unchecked(b"empty") },
            offset: 0,
            len: 0,
        }
    }
}

#[derive(Debug)]
#[repr(C)]
pub(crate) struct FileReferenceSet {
    entries: [UnsafeCell<Entry>; MAX_NUMBER_OF_ENTRIES],
    counter: [ReferenceCounter; MAX_NUMBER_OF_ENTRIES],
    decision_counter: [DecisionCounter; MAX_NUMBER_OF_ENTRIES],
    ids: FixedSizeUniqueIndexSet<MAX_NUMBER_OF_ENTRIES>,
    global_decision_counter: AtomicU64,
}

unsafe impl Send for FileReferenceSet {}
unsafe impl Sync for FileReferenceSet {}

impl Default for FileReferenceSet {
    fn default() -> Self {
        #[allow(clippy::declare_interior_mutable_const)]
        const COUNTER: ReferenceCounter = ReferenceCounter::new(0);
        #[allow(clippy::declare_interior_mutable_const)]
        const DEFAULT_ENTRY: UnsafeCell<Entry> = UnsafeCell::new(Entry::default());
        #[allow(clippy::declare_interior_mutable_const)]
        const DECISION: DecisionCounter = DecisionCounter::new();

        Self {
            entries: [DEFAULT_ENTRY; MAX_NUMBER_OF_ENTRIES],
            counter: [COUNTER; MAX_NUMBER_OF_ENTRIES],
            decision_counter: [DECISION; MAX_NUMBER_OF_ENTRIES],
            ids: FixedSizeUniqueIndexSet::new(),
            global_decision_counter: AtomicU64::new(0),
        }
    }
}

impl FileReferenceSet {
    pub(crate) fn insert(
        &self,
        name: &FileName,
        offset: usize,
        len: usize,
        is_persistent: bool,
    ) -> Result<FileReferenceSetId, SharedMemoryDirectoryCreateFileError> {
        let msg = "Unable to insert file";
        let id = match unsafe { self.ids.acquire_raw_index() } {
            Some(id) => id as usize,
            None => {
                fail!(from self,
                           with SharedMemoryDirectoryCreateFileError::FileLimitExceeded,
                           "{} \"{}\" into the set since there are no more entries available.", msg, *name);
            }
        };

        unsafe {
            self.entries[id].get().write(Entry {
                name: *name,
                offset,
                len,
            })
        };

        self.counter[id].set_persistency_bit(is_persistent);
        self.counter[id].increment_ref_counter();
        let current_decision_count = self.global_decision_counter.fetch_add(1, Ordering::Relaxed);
        if !self.decision_counter[id].set(current_decision_count) {
            fail!(from self, with SharedMemoryDirectoryCreateFileError::DoesExist,
                    "{} \"{}\" since the file already exists.", msg, *name);
        }

        // check for duplicates
        for i in 0..MAX_NUMBER_OF_ENTRIES {
            if i == id {
                continue;
            }

            if self.counter[i].increment_ref_counter_when_exist() {
                if unsafe { &*self.entries[i].get() }.name == *name
                    && !self.decision_counter[i].does_value_win(current_decision_count)
                {
                    if self.counter[i].is_initialized() {
                        fail!(from self, with SharedMemoryDirectoryCreateFileError::DoesExist,
                        "{} \"{}\" since the file already exists.", msg, *name);
                    } else {
                        fail!(from self, with SharedMemoryDirectoryCreateFileError::BeingCreated,
                        "{} \"{}\" since the file is currently being created.", msg, *name);
                    }
                }

                self.decrement_ref_counter(FileReferenceSetId(i));
            }
        }

        Ok(FileReferenceSetId(id))
    }

    // can only be called when the ownership is acquired
    pub(crate) fn to_be_removed(&self, name: &FileName) -> bool {
        if let Some(id) = self.find_entry(name) {
            self.counter[id.0].set_persistency_bit(false);
            self.counter[id.0].to_be_removed();
            self.decrement_ref_counter(id);
            return true;
        }

        false
    }

    // can only be called when the ownership is acquired
    pub(crate) fn finalize_initialization(&self, id: FileReferenceSetId) {
        self.counter[id.0].set_initialized_bit(true);
    }

    pub(crate) fn does_exist(&self, name: &FileName) -> bool {
        if let Some(id) = self.find_entry(name) {
            self.decrement_ref_counter(id);
            return true;
        }

        false
    }

    pub(crate) fn borrow(&self, name: &FileName, base_address: usize) -> Option<File> {
        self.find_entry(name).map(|id| File {
            set: self,
            id,
            base_address,
        })
    }

    // can only be called when the ownership is acquired
    pub(crate) fn release(&self, id: FileReferenceSetId) {
        self.decrement_ref_counter(id)
    }

    // can only be called when the ownership is acquired
    pub(crate) fn is_persistent(&self, id: FileReferenceSetId) -> bool {
        self.counter[id.0].is_persistent()
    }

    pub(crate) fn list(&self, base_address: usize) -> Vec<File> {
        let mut ret_val = vec![];
        for id in 0..self.ids.capacity() as usize {
            if self.counter[id].increment_ref_counter_when_initialized() {
                ret_val.push(File {
                    set: self,
                    id: FileReferenceSetId(id),
                    base_address,
                });
            }
        }

        ret_val
    }

    pub(crate) fn get_name(&self, id: FileReferenceSetId) -> FileName {
        unsafe { &*self.entries[id.0].get() }.name
    }

    pub(crate) fn get_payload(&self, id: FileReferenceSetId, base_address: usize) -> &[u8] {
        let entry_ref = unsafe { &*self.entries[id.0].get() };
        unsafe {
            core::slice::from_raw_parts(
                (entry_ref.offset + base_address) as *const u8,
                entry_ref.len,
            )
        }
    }

    #[allow(clippy::mut_from_ref)]
    pub(crate) fn get_payload_mut(&self, id: FileReferenceSetId, base_address: usize) -> &mut [u8] {
        let entry_ref = unsafe { &*self.entries[id.0].get() };
        unsafe {
            core::slice::from_raw_parts_mut(
                (entry_ref.offset + base_address) as *mut u8,
                entry_ref.len,
            )
        }
    }
    #[deny(clippy::mut_from_ref)]

    // if entry exists it acquires read-only ownership and returns the id
    fn find_entry(&self, name: &FileName) -> Option<FileReferenceSetId> {
        for id in 0..self.ids.capacity() as usize {
            if self.counter[id].increment_ref_counter_when_initialized() {
                if unsafe { *self.entries[id].get() }.name == *name {
                    return Some(FileReferenceSetId(id));
                }

                self.counter[id].decrement_ref_counter();
            }
        }

        None
    }

    fn decrement_ref_counter(&self, id: FileReferenceSetId) {
        if self.counter[id.0].decrement_ref_counter() {
            // remove entry
            self.counter[id.0].reset();
            self.decision_counter[id.0].set_to_undecided();
            unsafe { self.ids.release_raw_index(id.0 as u32) };
        }
    }
}
