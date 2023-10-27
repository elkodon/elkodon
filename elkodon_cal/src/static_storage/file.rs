//! File based implementation of [`StaticStorage`].
//!
//! # Example
//!
//! ```
//! use elkodon_bb_system_types::file_name::FileName;
//! use elkodon_bb_system_types::path::Path;
//! use elkodon_bb_container::semantic_string::SemanticString;
//! use elkodon_cal::static_storage::file::*;
//!
//! let mut content = "some storage content".to_string();
//! let custom_config = Configuration::default()
//!                         .suffix(FileName::new(b".conifg").unwrap())
//!                         .path_hint(Path::new(b"/tmp").unwrap());
//!
//! let storage_name = FileName::new(b"myStaticStorage").unwrap();
//! let owner = Builder::new(&storage_name)
//!                 .config(&custom_config)
//!                 .create(content.as_bytes()).unwrap();
//!
//! // usually a different process
//! let reader = Builder::new(&storage_name)
//!                 // if the config here differs the wrong static storage may be opened
//!                 .config(&custom_config)
//!                 .open().unwrap();
//!
//! let content_length = reader.len();
//! let mut content = String::from_utf8(vec![b' '; content_length as usize]).unwrap();
//! reader.read(unsafe { content.as_mut_vec() }.as_mut_slice()).unwrap();
//!
//! println!("Storage {} content: {}", reader.name(), content);
//! ```

pub use crate::named_concept::*;
pub use crate::static_storage::*;

use elkodon_bb_log::{error, fail, warn};
use elkodon_bb_posix::{
    directory::*, file::*, file_descriptor::FileDescriptorManagement, file_type::FileType,
};

const FINAL_PERMISSIONS: Permission = Permission::OWNER_READ;

/// The custom configuration of [``].
#[derive(Clone, Debug)]
pub struct Configuration {
    path: Path,
    suffix: FileName,
}

impl Default for Configuration {
    fn default() -> Self {
        Configuration {
            path: DEFAULT_PATH_HINT,
            suffix: DEFAULT_SUFFIX,
        }
    }
}

impl crate::named_concept::NamedConceptConfiguration for Configuration {
    fn suffix(mut self, value: FileName) -> Self {
        self.suffix = value;
        self
    }

    fn path_hint(mut self, value: Path) -> Self {
        self.path = value;
        self
    }

    fn get_suffix(&self) -> &FileName {
        &self.suffix
    }

    fn get_path_hint(&self) -> &Path {
        &self.path
    }
}

impl crate::static_storage::StaticStorageConfiguration for Configuration {}

#[derive(Debug)]
pub struct Locked {
    static_storage: Storage,
}

impl NamedConcept for Locked {
    fn name(&self) -> &FileName {
        self.static_storage.name()
    }
}

impl StaticStorageLocked<Storage> for Locked {
    fn unlock(mut self, contents: &[u8]) -> Result<Storage, StaticStorageUnlockError> {
        let msg = "Failed to unlock storage";
        let bytes_written = fail!(from self, when self.static_storage.file.write(contents),
            map FileWriteError::InsufficientPermissions => StaticStorageUnlockError::InsufficientPermissions;
                FileWriteError::NoSpaceLeft => StaticStorageUnlockError::NoSpaceLeft,
            unmatched StaticStorageUnlockError::InternalError,
            "{} due to a failure while writing the contents.", msg);

        if bytes_written != contents.len() as u64 {
            fail!(from self, with StaticStorageUnlockError::NoSpaceLeft,
                "{} since the contents length is {} bytes but only {} bytes could be written to the file.",
                msg, contents.len(), bytes_written);
        }

        fail!(from self, when self.static_storage.file.set_permission(FINAL_PERMISSIONS),
                map FileSetPermissionError::InsufficientPermissions => StaticStorageUnlockError::InsufficientPermissions,
                unmatched StaticStorageUnlockError::InternalError,
                "{} due to a failure while updating the permissions to {}.", msg, FINAL_PERMISSIONS);

        self.static_storage.len = contents.len() as u64;

        Ok(self.static_storage)
    }
}

/// Implements [`StaticStorage`] for a file.
#[derive(Debug)]
pub struct Storage {
    name: FileName,
    config: Configuration,
    has_ownership: bool,
    file: File,
    len: u64,
}

impl Drop for Storage {
    fn drop(&mut self) {
        if self.has_ownership {
            if let Err(v) = unsafe { Self::remove_cfg(&self.name, &self.config) } {
                warn!(from self, "Unable to remove owned static storage due to ({:?}). This may cause a leak!", v);
            }
        }
    }
}

impl crate::named_concept::NamedConcept for Storage {
    fn name(&self) -> &FileName {
        &self.name
    }
}

impl crate::named_concept::NamedConceptMgmt for Storage {
    type Configuration = Configuration;

    unsafe fn remove_cfg(
        storage_name: &FileName,
        config: &Self::Configuration,
    ) -> Result<bool, NamedConceptRemoveError> {
        let msg = format!("Unable to release static storage \"{}\"", storage_name);
        let origin = "static_storage::file::Storage::remove_cfg()";

        let file_path = config.path_for(storage_name);
        match File::remove(&file_path) {
            Ok(v) => Ok(v),
            Err(FileRemoveError::InsufficientPermissions)
            | Err(FileRemoveError::PartOfReadOnlyFileSystem) => {
                fail!(from origin, with NamedConceptRemoveError::InsufficientPermissions,
                        "{} due to insufficient permissions.", msg);
            }
            Err(v) => {
                fail!(from origin, with NamedConceptRemoveError::InternalError,
                        "{} due to unknown failure ({:?}).", msg, v);
            }
        }
    }

    fn list_cfg(config: &Configuration) -> Result<Vec<FileName>, NamedConceptListError> {
        let msg = "Unable to list all storages";
        let origin = "static_storage::File::list_cfg()";
        let directory = fail!(from origin, when Directory::new(&config.path),
            map DirectoryOpenError::InsufficientPermissions => NamedConceptListError::InsufficientPermissions,
            unmatched NamedConceptListError::InternalError,
            "{} due to a failure while reading the storage directory (\"{}\").", msg, config.path);

        let entries = fail!(from origin,
                            when directory.contents(),
                            map DirectoryReadError::InsufficientPermissions => NamedConceptListError::InsufficientPermissions,
                            unmatched NamedConceptListError::InternalError,
                            "{} due to a failure while reading the storage directory (\"{}\") contents.", msg, config.path);

        let mut result = vec![];
        for entry in &entries {
            let metadata = entry.metadata();
            if metadata.file_type() == FileType::File && metadata.permission() == FINAL_PERMISSIONS
            {
                if let Some(entry_name) = config.extract_name_from_file(entry.name()) {
                    result.push(entry_name);
                }
            }
        }

        Ok(result)
    }

    fn does_exist_cfg(
        storage_name: &FileName,
        config: &Configuration,
    ) -> Result<bool, NamedConceptDoesExistError> {
        let msg = format!("Unable to check if storage \"{}\" exists", storage_name);
        let origin = "static_storage::file::Storage::does_exist_cfg()";

        let adjusted_path = config.path_for(storage_name);

        match File::does_exist(&adjusted_path) {
            Ok(true) => (),
            Ok(false) => return Ok(false),
            Err(v) => {
                fail!(from origin, with NamedConceptDoesExistError::UnderlyingResourcesCorrupted,
                    "{} due to an internal failure ({:?}), is the static storage in a corrupted state?", msg, v);
            }
        };

        let file = FileBuilder::new(&adjusted_path).open_existing(AccessMode::Read);
        if file.is_err() {
            fail!(from origin, with NamedConceptDoesExistError::UnderlyingResourcesCorrupted,
                "{} since the file could not be opened for reading ({:?}), is static storage in a corrupted state?", msg, file.err().unwrap() );
        }

        let file = file.unwrap();
        let metadata = file.metadata();
        if metadata.is_err() {
            fail!(from origin, with NamedConceptDoesExistError::UnderlyingResourcesCorrupted,
                "{} due to an internal failure ({:?}) while acquiring underlying file informations, is static storage in a corrupted state?",
                msg, metadata.err().unwrap());
        }
        let metadata = metadata.unwrap();

        if metadata.file_type() == FileType::File && metadata.permission() == FINAL_PERMISSIONS {
            return Ok(true);
        }

        fail!(from origin, with NamedConceptDoesExistError::UnderlyingResourcesBeingSetUp,
                "{} since the underlying resources are currently being created or the creation process hangs.", msg);
    }
}

impl crate::static_storage::StaticStorage for Storage {
    type Builder = Builder;
    type Locked = Locked;

    fn release_ownership(&mut self) {
        self.has_ownership = false
    }

    fn acquire_ownership(&mut self) {
        self.has_ownership = true
    }

    fn len(&self) -> u64 {
        self.len
    }

    fn is_empty(&self) -> bool {
        self.len == 0
    }

    fn read(&self, content: &mut [u8]) -> Result<(), StaticStorageReadError> {
        let msg = "Unable to read from static storage";
        let len = self.len();

        if len > content.len() as u64 {
            fail!(from self, with StaticStorageReadError::BufferTooSmall,
                "{} since a buffer with a size of a least {} bytes is required to read the file but a buffer of size {} bytes was provided.",
                msg, len, content.len());
        }

        let bytes_read = fail!(from self, when self.file.read(content),
                                with StaticStorageReadError::ReadError,
                                "{} due to a failure while reading the underlying file.", msg);

        if bytes_read != len {
            fail!(from self, with StaticStorageReadError::StaticStorageWasModified,
                        "{} since the expected read size is {} bytes but {} bytes were read instead. Was the static storage file modified?",
                        msg, len, bytes_read);
        }

        Ok(())
    }
}

/// Creates [``] which owns the file and removes it when going out of scope
/// or [`Reader`].
#[derive(Debug)]
pub struct Builder {
    storage_name: FileName,
    has_ownership: bool,
    config: Configuration,
}

impl crate::named_concept::NamedConceptBuilder<Storage> for Builder {
    fn new(storage_name: &FileName) -> Self {
        Self {
            storage_name: *storage_name,
            has_ownership: true,
            config: <Configuration as Default>::default(),
        }
    }

    fn config(mut self, config: &Configuration) -> Self {
        self.config = config.clone();
        self
    }
}

impl crate::static_storage::StaticStorageBuilder<Storage> for Builder {
    fn has_ownership(mut self, value: bool) -> Self {
        self.has_ownership = value;
        self
    }

    fn create_locked(self) -> Result<Locked, StaticStorageCreateError> {
        let directory_permission = Permission::OWNER_ALL | Permission::GROUP_ALL;

        let msg = format!("Unable to create target directory \"{}\"", self.config.path);
        if !fail!(from self, when Directory::does_exist(&self.config.path),
            with StaticStorageCreateError::Creation,
               "{} since the system is unable to determine if the directory even exists.", msg)
        {
            fail!(from self, when Directory::create(&self.config.path, directory_permission ),
                with StaticStorageCreateError::Creation,
                "{} due to a failure while creating the service root directory.", msg);
            error!(from self, "Created service root directory \"{}\" since it did not exist before.", self.config.path);
        }

        let file = fail!(from self, when
            FileBuilder::new(&self.config.path_for(&self.storage_name))
            .creation_mode(CreationMode::CreateExclusive)
            .permission(Permission::OWNER_ALL)
            .create(),
            map FileCreationError::FileAlreadyExists => StaticStorageCreateError::AlreadyExists;
                FileCreationError::InsufficientPermissions => StaticStorageCreateError::InsufficientPermissions,
            unmatched StaticStorageCreateError::Creation,
            "{} due to a failure while creating the underlying file.", msg);

        Ok(Locked {
            static_storage: Storage {
                name: self.storage_name,
                config: self.config,
                has_ownership: self.has_ownership,
                file,
                len: 0,
            },
        })
    }

    fn open(self) -> Result<Storage, StaticStorageOpenError> {
        let msg = "Unable to open static storage";
        let origin = "static_storage::File::Reader::new()";

        let file = fail!(from origin,
            when FileBuilder::new(&self.config.path_for(&self.storage_name)).open_existing(AccessMode::Read),
            with StaticStorageOpenError::DoesNotExist,
            "{} due to a failure while opening the file.", msg);

        let metadata = fail!(from origin,
            when file.metadata(), with StaticStorageOpenError::Read,
            "{} due to a failure while reading the files metadata.", msg);

        if metadata.permission() != FINAL_PERMISSIONS {
            fail!(from origin, with StaticStorageOpenError::IsLocked,
                "{} since the static storage is still being created (in locked state), try later.", msg);
        }

        Ok(Storage {
            name: self.storage_name,
            config: self.config,
            has_ownership: self.has_ownership,
            file,
            len: metadata.size(),
        })
    }
}
