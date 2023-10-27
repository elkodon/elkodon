mod decision_counter;
pub mod file;
mod file_reference_set;
mod reference_counter;

use crate::shared_memory_directory::file_reference_set::FileReferenceSet;
use crate::shm_allocator::bump_allocator::BumpAllocator;
use crate::{named_concept::*, shared_memory::*, shm_allocator::ShmAllocator};
use elkodon_bb_elementary::math::align_to;
use elkodon_bb_log::{fail, fatal_panic};
use elkodon_bb_system_types::file_name::FileName;
use std::{alloc::Layout, fmt::Debug, marker::PhantomData};

use crate::shared_memory_directory::file::{File, FileCreator};

const MAX_NUMBER_OF_ENTRIES: usize = 512;
const MGMT_SHM_SUFFIX: FileName = unsafe { FileName::new_unchecked(b".dm") };
const DATA_SHM_SUFFIX: FileName = unsafe { FileName::new_unchecked(b".dd") };

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SharedMemoryDirectoryCreateFileError {
    FileLimitExceeded,
    BeingCreated,
    DoesExist,
}

#[derive(Debug)]
pub struct SharedMemoryDirectoryCreator {
    name: FileName,
    size: usize,
    is_persistent: bool,
}

impl SharedMemoryDirectoryCreator {
    pub fn new(name: &FileName) -> Self {
        Self {
            name: *name,
            size: 0,
            is_persistent: false,
        }
    }

    pub fn is_persistent(mut self, value: bool) -> Self {
        self.is_persistent = value;
        self
    }

    pub fn size(mut self, value: usize) -> Self {
        self.size = value;
        self
    }

    pub fn create<
        MgmtShm: SharedMemory<BumpAllocator>,
        Allocator: ShmAllocator,
        DataShm: SharedMemory<Allocator>,
    >(
        self,
        allocator_config: &Allocator::Configuration,
    ) -> Result<SharedMemoryDirectory<MgmtShm, Allocator, DataShm>, SharedMemoryCreateError> {
        let msg = "Unable to create shared memory directory";
        let mut mgmt_shm = fail!(from self,
        when MgmtShm::Builder::new(&self.name)
            .config(
                &MgmtShm::Configuration::default()
                    .suffix(MGMT_SHM_SUFFIX),
            )
            .size(core::mem::size_of::<FileReferenceSet>() + core::mem::align_of::<FileReferenceSet>() - 1)
            .create(&<BumpAllocator as ShmAllocator>::Configuration::default()),
        "{} since the management segment could not be created.", msg);

        let shm_ptr = fatal_panic!(from self,
                                when mgmt_shm.allocate(std::alloc::Layout::new::<FileReferenceSet>()),
                                "This should never happen! {} since the allocation of the management segment failed.",
                                msg);

        let files = shm_ptr.data_ptr as *mut FileReferenceSet;
        unsafe { files.write(FileReferenceSet::default()) };

        let mut data_shm = fail!(from self,
            when DataShm::Builder::new(&self.name).config(
                &DataShm::Configuration::default()
                    .suffix(DATA_SHM_SUFFIX),
                ).size(self.size).create(allocator_config),
            "{} since the data segment could not be created.", msg);

        if self.is_persistent {
            mgmt_shm.release_ownership();
            data_shm.release_ownership();
        }

        Ok(SharedMemoryDirectory {
            _mgmt_shm: mgmt_shm,
            data_shm,
            files,
            _allocator: PhantomData,
        })
    }

    pub fn open<
        MgmtShm: SharedMemory<BumpAllocator>,
        Allocator: ShmAllocator,
        DataShm: SharedMemory<Allocator>,
    >(
        self,
    ) -> Result<SharedMemoryDirectory<MgmtShm, Allocator, DataShm>, SharedMemoryOpenError> {
        let msg = "Unable to open shared memory directory";
        let data_shm = fail!(from self, when DataShm::Builder::new(&self.name)
                                .config(
                                    &DataShm::Configuration::default()
                                        .suffix(DATA_SHM_SUFFIX),
                                    )
                                .open(),
                                "{} since the data segment could not be opened.", msg);

        let mgmt_shm = fail!(from self, when MgmtShm::Builder::new(&self.name)
                                .config(
                                    &MgmtShm::Configuration::default()
                                        .suffix(MGMT_SHM_SUFFIX),
                                )
                                .open(),
                                "{} since the management segment could not be opened.", msg);

        let files = align_to::<FileReferenceSet>(mgmt_shm.allocator_data_start_address())
            as *mut FileReferenceSet;

        Ok(SharedMemoryDirectory {
            _mgmt_shm: mgmt_shm,
            data_shm,
            files,
            _allocator: PhantomData,
        })
    }
}

pub struct SharedMemoryDirectory<
    MgmtShm: SharedMemory<BumpAllocator>,
    Allocator: ShmAllocator,
    DataShm: SharedMemory<Allocator>,
> {
    _mgmt_shm: MgmtShm,
    data_shm: DataShm,
    files: *mut FileReferenceSet,
    _allocator: PhantomData<Allocator>,
}

impl<
        MgmtShm: SharedMemory<BumpAllocator>,
        Allocator: ShmAllocator,
        DataShm: SharedMemory<Allocator>,
    > Debug for SharedMemoryDirectory<MgmtShm, Allocator, DataShm>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SharedMemoryDirectory {{  }}")
    }
}

impl<
        MgmtShm: SharedMemory<BumpAllocator>,
        Allocator: ShmAllocator,
        DataShm: SharedMemory<Allocator>,
    > SharedMemoryDirectory<MgmtShm, Allocator, DataShm>
{
    pub fn new_file(&self, layout: Layout) -> Result<FileCreator, ShmAllocationError> {
        let memory = fail!(from self, when self.data_shm.allocate(layout),
            "Unable to create file since the allocation of {:?} failed.", layout);

        Ok(FileCreator::new(
            self.files(),
            memory,
            layout,
            self.data_shm.allocator_data_start_address(),
        ))
    }

    pub fn open_file(&self, name: &FileName) -> Option<File> {
        self.files()
            .borrow(name, self.data_shm.allocator_data_start_address())
    }

    pub fn list_files(&self) -> Vec<File> {
        self.files()
            .list(self.data_shm.allocator_data_start_address())
    }

    pub fn does_file_exist(&self, name: &FileName) -> bool {
        self.files().does_exist(name)
    }

    pub fn remove_file(&self, name: &FileName) -> bool {
        self.files().to_be_removed(name)
    }

    pub fn file_capacity(&self) -> usize {
        MAX_NUMBER_OF_ENTRIES
    }

    pub fn memory_capacity(&self) -> usize {
        self.data_shm.size()
    }

    pub fn does_exist(name: &FileName) -> Result<bool, NamedConceptDoesExistError> {
        let msg = "Unable to check if the SharedMemoryDirectory";
        let origin = "SharedMemoryDirectory::does_exist()";

        if !fail!(from origin, when DataShm::does_exist_cfg(
            name,
            &DataShm::Configuration::default().suffix(DATA_SHM_SUFFIX)),
            "{} \"{}\" exists due to a failure while checking the data segment.", msg, name)
        {
            return Ok(false);
        }

        let mgmt_result = fail!(from origin,
            when MgmtShm::does_exist_cfg(name, &MgmtShm::Configuration::default().suffix(MGMT_SHM_SUFFIX)),
            "{} \"{}\" exists due to a failure while checking the management segment.", msg, name
        );

        Ok(mgmt_result)
    }

    pub fn list() -> Result<Vec<FileName>, NamedConceptListError> {
        let msg = "Unable to list all SharedMemoryDirectories";
        let origin = "SharedMemoryDirectory::list()";

        Ok(fail!(from origin, when DataShm::list_cfg(
            &DataShm::Configuration::default().suffix(DATA_SHM_SUFFIX)
                ),
            "{} since the data segments could not be listed.", msg))
    }

    /// # Safety
    ///   * The [`SharedMemoryDirectory`] shall not be used by any other process otherwise
    ///     other instances are working on a stale [`SharedMemoryDirectory`] instance
    pub unsafe fn remove(name: &FileName) -> Result<bool, NamedConceptRemoveError> {
        let msg = "Unable to remove SharedMemoryDirectory";
        let origin = "SharedMemoryDirectory::remove()";

        if !fail!(from origin, when DataShm::remove_cfg(
            name,
            &DataShm::Configuration::default().suffix(DATA_SHM_SUFFIX)),
            "{} \"{}\" since the data segment could not be removed.", msg, name)
        {
            return Ok(false);
        }

        let mgmt_result = fail!(from origin,
            when MgmtShm::remove_cfg(name, &MgmtShm::Configuration::default().suffix(MGMT_SHM_SUFFIX)),
            "{} \"{}\" since the management segment could not be removed.", msg, name
        );

        Ok(mgmt_result)
    }

    pub fn size(&self) -> usize {
        self.data_shm.size()
    }

    fn files(&self) -> &FileReferenceSet {
        unsafe { &*self.files }
    }
}
