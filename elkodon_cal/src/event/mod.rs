pub mod process_local;
pub mod unix_datagram_socket;

use std::{fmt::Debug, time::Duration};

pub use crate::named_concept::{NamedConcept, NamedConceptBuilder, NamedConceptMgmt};
use elkodon_bb_posix::config::TEMP_DIRECTORY;
pub use elkodon_bb_system_types::file_name::FileName;
pub use elkodon_bb_system_types::path::Path;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NotifierNotifyError {
    FailedToDeliverSignal,
    InternalFailure,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NotifierCreateError {
    DoesNotExist,
    InsufficientPermissions,
    InternalFailure,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ListenerWaitError {
    ContractViolation,
    InternalFailure,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ListenerCreateError {
    AlreadyExists,
    InsufficientPermissions,
    InternalFailure,
}

/// The default suffix of every event
pub const DEFAULT_SUFFIX: FileName = unsafe { FileName::new_unchecked(b".event") };

/// The default path hint for every event
pub const DEFAULT_PATH_HINT: Path = TEMP_DIRECTORY;

pub trait TriggerId: Debug + Copy {}

impl TriggerId for u64 {}
impl TriggerId for u32 {}
impl TriggerId for u16 {}
impl TriggerId for u8 {}

pub trait Notifier<Id: TriggerId>: NamedConcept + Debug {
    fn notify(&self, id: Id) -> Result<(), NotifierNotifyError>;
}

pub trait NotifierBuilder<Id: TriggerId, T: Event<Id>>: NamedConceptBuilder<T> + Debug {
    fn open(self) -> Result<T::Notifier, NotifierCreateError>;
}

pub trait Listener<Id: TriggerId>: NamedConcept + Debug {
    fn try_wait(&self) -> Result<Option<Id>, ListenerWaitError>;
    fn timed_wait(&self, timeout: Duration) -> Result<Option<Id>, ListenerWaitError>;
    fn blocking_wait(&self) -> Result<Option<Id>, ListenerWaitError>;
}

pub trait ListenerBuilder<Id: TriggerId, T: Event<Id>>: NamedConceptBuilder<T> + Debug {
    fn create(self) -> Result<T::Listener, ListenerCreateError>;
}

pub trait Event<Id: TriggerId>: Sized + NamedConceptMgmt + Debug {
    type Notifier: Notifier<Id>;
    type NotifierBuilder: NotifierBuilder<Id, Self>;
    type Listener: Listener<Id>;
    type ListenerBuilder: ListenerBuilder<Id, Self>;
}
