use std::fmt::Debug;

use elkodon_bb_container::semantic_string::SemanticString;
use elkodon_bb_log::fatal_panic;
pub use elkodon_bb_system_types::file_name::FileName;
pub use elkodon_bb_system_types::file_path::FilePath;
pub use elkodon_bb_system_types::path::Path;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum NamedConceptDoesExistError {
    InsufficientPermissions,
    UnderlyingResourcesBeingSetUp,
    UnderlyingResourcesCorrupted,
    InternalError,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum NamedConceptRemoveError {
    InsufficientPermissions,
    InternalError,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq)]
pub enum NamedConceptListError {
    InsufficientPermissions,
    InternalError,
}

/// Every [`NamedConcept`] must have a custom configuration that at least allows the user to define
/// a custom [`NamedConceptConfiguration::suffix()`] for all file names that are transparent during
/// usage as well as a [`NamedConceptConfiguration::path_hint()`] that can be ignored if the
/// underlying resource does not support it.
pub trait NamedConceptConfiguration: Default + Clone + Debug {
    /// Defines the suffix that the concept will use.
    fn suffix(self, value: FileName) -> Self;

    /// Sets a path hint under which the underlying resources shall be stored. When the concept
    /// uses resources like [`elkodon_bb_posix::shared_memory::SharedMemory`] the path will be
    /// ignored.
    fn path_hint(self, value: Path) -> Self;

    /// Returns the configurations suffix.
    fn get_suffix(&self) -> &FileName;

    /// Returns the configurations path hint.
    fn get_path_hint(&self) -> &Path;

    /// Returns the full path for a given value under the given configuration.
    fn path_for(&self, value: &FileName) -> FilePath {
        let mut path = *self.get_path_hint();
        fatal_panic!(from self, when path.add_path_entry(value),
                    "The path hint \"{}\" in combination with the file name \"{}\" exceed the maximum supported path length of {} of the operating system.",
                    path, value, Path::max_len());
        fatal_panic!(from self, when path.push_bytes(self.get_suffix()),
                    "The path hint \"{}\" in combination with the file name \"{}\" and the suffix \"{}\" exceed the maximum supported path length of {} of the operating system.",
                    path, value, self.get_suffix(), Path::max_len());

        unsafe { FilePath::new_unchecked(path.as_bytes()) }
    }

    /// Extracts the name from a full path under a given configuration.
    fn extract_name_from_path(&self, value: &FilePath) -> Option<FileName> {
        if *self.get_path_hint() != value.path() {
            return None;
        }

        let mut file = unsafe { FileName::new_unchecked(value.file_name()) };
        let strip_result = fatal_panic!(from self, when file.strip_suffix(self.get_suffix().as_bytes()),
                    "Stripping the suffix \"{}\" from the file name \"{}\" leads to invalid content.",
                    self.get_suffix(), file);

        if !strip_result {
            return None;
        }

        Some(file)
    }

    /// Extracts the name from a file name under a given configuration.
    fn extract_name_from_file(&self, value: &FileName) -> Option<FileName> {
        let mut file = *value;
        let strip_result = fatal_panic!(from self, when file.strip_suffix(self.get_suffix().as_bytes()),
                    "Stripping the suffix \"{}\" from the file name \"{}\" leads to invalid content.",
                    self.get_suffix(), file);

        if !strip_result {
            return None;
        }

        Some(file)
    }
}

/// Builder trait to create new [`NamedConcept`]s.
pub trait NamedConceptBuilder<T: NamedConceptMgmt> {
    /// Defines the name of the newly created [`NamedConcept`].
    fn new(name: &FileName) -> Self;

    /// Sets the custom configuration of the concept.
    fn config(self, config: &T::Configuration) -> Self;
}

/// Every concept that is uniquely identified by a [`FileName`] and corresponds to some kind of
/// file in the file system is a [`NamedConcept`]. This trait provides the essential property of
/// these concepts [`NamedConcept::name()`]
pub trait NamedConcept {
    /// Returns the name of the concept
    fn name(&self) -> &FileName;
}

/// Every concept that is uniquely identified by a [`FileName`] and corresponds to some kind of
/// file in the file system is a [`NamedConcept`]. This trait provides common management methods
/// for such concepts, like
///  * [`NamedConceptMgmt::remove()`]
///  * [`NamedConceptMgmt::does_exist()`]
///  * [`NamedConceptMgmt::list()`]
pub trait NamedConceptMgmt {
    type Configuration: NamedConceptConfiguration;

    /// Removes an existing concept. Returns true if the concepts existed and was removed,
    /// if the concept did not exist it returns false.
    ///
    /// # Safety
    ///
    ///  * It must be ensured that no other process is using the concept.
    ///
    unsafe fn remove(name: &FileName) -> Result<bool, NamedConceptRemoveError> {
        Self::remove_cfg(name, &Self::Configuration::default())
    }

    /// Returns true if a concept with that name exists, otherwise false
    fn does_exist(name: &FileName) -> Result<bool, NamedConceptDoesExistError> {
        Self::does_exist_cfg(name, &Self::Configuration::default())
    }

    /// Returns a list of all available concepts with the default configuration.
    fn list() -> Result<Vec<FileName>, NamedConceptListError> {
        Self::list_cfg(&Self::Configuration::default())
    }

    /// Removes an existing concept under a custom configuration. Returns true if the concepts
    /// existed and was removed, if the concept did not exist it returns false.
    ///
    /// # Safety
    ///
    ///  * It must be ensured that no other process is using the concept.
    ///
    unsafe fn remove_cfg(
        name: &FileName,
        cfg: &Self::Configuration,
    ) -> Result<bool, NamedConceptRemoveError>;

    /// Returns true if a concept with that name exists under a custom configuration, otherwise false
    fn does_exist_cfg(
        name: &FileName,
        cfg: &Self::Configuration,
    ) -> Result<bool, NamedConceptDoesExistError>;

    /// Returns a list of all available concepts with a custom configuration.
    fn list_cfg(cfg: &Self::Configuration) -> Result<Vec<FileName>, NamedConceptListError>;
}
