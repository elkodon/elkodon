//! A FileLock can be created around any object which implements the [`FileDescriptorBased`] trait.
//! Either one can exclusively lock the file for writing or many can lock it for reading.
//!
//! # Example
//!
//! ```
//! use elkodon_bb_posix::file::*;
//! use elkodon_bb_posix::file_lock::*;
//! use elkodon_bb_system_types::file_path::FilePath;
//! use elkodon_bb_container::semantic_string::SemanticString;
//!
//! let file_name = FilePath::new(b"/tmp/file_lock_demo1").unwrap();
//! let file = FileBuilder::new(&file_name)
//!                              .creation_mode(CreationMode::PurgeAndCreate)
//!                              .permission(Permission::OWNER_ALL)
//!                              .create()
//!                              .expect("failed to create file");
//!
//! let handle = ReadWriteMutexHandle::new();
//! let fileWithLock = FileLockBuilder::new().clock_type(ClockType::Monotonic)
//!                                          .create(file, &handle).expect("failed to create lock");
//!
//! fileWithLock.write_lock().unwrap().write(b"Hello world!");
//! let mut content = String::new();
//! fileWithLock.read_lock().unwrap().read_to_string(&mut content);
//! ```

pub use crate::read_write_mutex::ReadWriteMutexHandle;

use crate::file_descriptor::FileDescriptor;
use crate::process::{Process, ProcessId};
use crate::read_write_mutex::*;
use crate::{clock::Time, file_descriptor::FileDescriptorBased};
use elkodon_bb_elementary::enum_gen;
use elkodon_bb_log::fail;
use elkodon_pal_posix::posix::errno::Errno;
use elkodon_pal_posix::posix::Struct;
use elkodon_pal_posix::*;
use std::fmt::Debug;
use std::sync::atomic::{AtomicI64, Ordering};
use std::{ops::Deref, ops::DerefMut, time::Duration};

use crate::{
    adaptive_wait::*,
    clock::{NanosleepError, TimeError},
};

pub use crate::clock::ClockType;

enum_gen! { FileWriterGetLockError
  mapping:
    FileTryLockError,
    NanosleepError,
    ReadWriteMutexWriteLockError
}

enum_gen! { FileReaderGetLockError
  mapping:
    FileTryLockError,
    NanosleepError,
    ReadWriteMutexReadLockError
}

enum_gen! { FileTryLockError
  entry:
    Interrupt,
    ExceedsMaximumNumberOfLockedRegionsInSystem,
    InvalidFileDescriptorOrWrongOpenMode,
    DeadlockConditionDetected,
    UnknownError(i32)
}

enum_gen! { FileWriterTryLockError
  mapping:
    FileTryLockError,
    ReadWriteMutexWriteLockError
}

enum_gen! { FileReaderTryLockError
  mapping:
    FileTryLockError,
    ReadWriteMutexReadLockError
}

enum_gen! { FileUnlockError
  entry:
    Interrupt,
    InvalidFileDescriptorOrWrongOpenMode,
    IsNotLocked,
    UnknownError(i32)
}

enum_gen! { FileLockStateError
  entry:
    InvalidFileDescriptor,
    Interrupt,
    UnknownError(i32)

  mapping:
    ReadWriteMutexReadLockError
}

enum_gen! { FileTimedLockError
  mapping:
    FileTryLockError,
    TimeError,
    AdaptiveWaitError
}

enum_gen! { FileWriterTimedLockError
  mapping:
    TimeError,
    FileTimedLockError,
    ReadWriteMutexWriteTimedLockError

}

enum_gen! { FileReaderTimedLockError
  mapping:
    TimeError,
    FileTimedLockError,
    ReadWriteMutexReadTimedLockError

}

enum_gen! {
    /// The FileLockError enum is a generalization when one doesn't require the fine-grained error
    /// handling enums. One can forward FileLockError as more generic return value when a method
    /// returns a FileLock***Error.
    /// On a higher level it is again convertable to [`crate::Error`]
    FileLockError
  generalization:
    UnableToAcquireLock <= FileWriterGetLockError; FileReaderGetLockError; FileTryLockError; FileWriterTryLockError; FileReaderTryLockError; FileUnlockError; FileLockStateError;FileTimedLockError; FileWriterTimedLockError; FileReaderTimedLockError
}

/// A guard which is acquired when the file could be successfully locked for writing with
/// [`FileLock::write_lock()`], [`FileLock::write_try_lock()`] or [`FileLock::write_timed_lock()`].
/// It provides read and write access to the underlying file and unlocks it as soon as it goes out
/// of scope.
#[derive(Debug)]
pub struct FileLockWriteGuard<'a, 'b, T: FileDescriptorBased + Debug> {
    file_lock: &'a FileLock<'b, T>,
    guard: MutexWriteGuard<'a, 'b, T>,
}

unsafe impl<T: Send + FileDescriptorBased + Debug> Send for FileLockWriteGuard<'_, '_, T> {}
unsafe impl<T: Send + Sync + FileDescriptorBased + Debug> Sync for FileLockWriteGuard<'_, '_, T> {}

impl<T: FileDescriptorBased + Debug> Deref for FileLockWriteGuard<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T: FileDescriptorBased + Debug> DerefMut for FileLockWriteGuard<'_, '_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl<T: FileDescriptorBased + Debug> Drop for FileLockWriteGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.file_lock.release(self.guard.file_descriptor()).ok();
    }
}

/// A guard which is acquired when the file could be successfully locked for reading with
/// [`FileLock::read_lock()`], [`FileLock::read_try_lock()`] or [`FileLock::read_timed_lock()`].
/// It provides read access to the underlying file and unlocks it as soon as it goes out
/// of scope.
#[derive(Debug)]
pub struct FileLockReadGuard<'a, 'b, T: FileDescriptorBased + Debug> {
    file_lock: &'a FileLock<'b, T>,
    guard: MutexReadGuard<'a, 'b, T>,
}

unsafe impl<T: Send + FileDescriptorBased + Debug> Send for FileLockReadGuard<'_, '_, T> {}
unsafe impl<T: Send + Sync + FileDescriptorBased + Debug> Sync for FileLockReadGuard<'_, '_, T> {}

impl<T: FileDescriptorBased + Debug> Deref for FileLockReadGuard<'_, '_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl<T: FileDescriptorBased + Debug> Drop for FileLockReadGuard<'_, '_, T> {
    fn drop(&mut self) {
        self.file_lock.release(self.guard.file_descriptor()).ok();
    }
}

/// The builder to create a [`FileLock`].
///
/// One has to create an object first which implements the [`FileDescriptorBased`] trait.
///
#[derive(Debug)]
pub struct FileLockBuilder {
    clock_type: ClockType,
    priority: ReadWriteMutexPriority,
}

impl Default for FileLockBuilder {
    fn default() -> Self {
        Self {
            clock_type: ClockType::default(),
            priority: ReadWriteMutexPriority::PreferReader,
        }
    }
}

impl FileLockBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the clock type used for [`FileLock::read_timed_lock()`] or
    /// [`FileLock::write_timed_lock()`].
    ///
    /// **Attention!** Be aware that [`ClockType::Realtime`] can cause deadlocks when the time
    /// of the system is updated to a timepoint in the past.
    pub fn clock_type(mut self, value: ClockType) -> Self {
        self.clock_type = value;
        self
    }

    /// Sets the priority and selects if reader locks or writer locks should be prioritized
    pub fn priority(mut self, value: ReadWriteMutexPriority) -> Self {
        self.priority = value;
        self
    }

    pub fn create<T: FileDescriptorBased + Debug>(
        self,
        value: T,
        handle: &ReadWriteMutexHandle<T>,
    ) -> Result<FileLock<T>, ReadWriteMutexCreationError> {
        FileLock::new(value, self, handle)
    }
}

/// A FileLock can be created around any object which implements the [`FileDescriptorBased`] trait.
/// Either one can exclusively lock the file for writing or many can lock it for reading.
///
/// **Attention!** The FileLock is and advisary lock and not secure! If all participants follow the
/// rules it provides threadsafe access but if one accesses the underlying file directly one can
/// experience race-conditions.
///
/// **Attention!** Only the same instance of FileLock blocks inside the same process. If one for
/// instance opens the file twice in the same process the read/write locking will not work anymore.
/// But it works between processes. If two different processes are opening the same file
/// read/writer locks will block the processes.
#[derive(Debug)]
pub struct FileLock<'a, T: FileDescriptorBased + Debug> {
    file: ReadWriteMutex<'a, T>,
    clock_type: ClockType,
    lock_state: AtomicI64,
}

unsafe impl<T: Send + FileDescriptorBased + Debug> Send for FileLock<'_, T> {}
unsafe impl<T: Send + Sync + FileDescriptorBased + Debug> Sync for FileLock<'_, T> {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i16)]
pub enum LockType {
    Read = posix::F_RDLCK as i16,
    Write = posix::F_WRLCK as i16,
    Unlock = posix::F_UNLCK as i16,
}

/// Describes the current state of the [`FileLock`]. If no one holds the lock then
/// [`LockType::Unlock`] is set, otherwise [`LockType::Read`] or [`LockType::Write`] and the
/// process id of the owner of the lock.
#[derive(Debug)]
pub struct LockState {
    lock_type: LockType,
    pid: ProcessId,
}

impl LockState {
    pub fn lock_type(&self) -> LockType {
        self.lock_type
    }

    pub fn pid_of_owner(&self) -> ProcessId {
        self.pid
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InternalMode {
    Blocking,
    NonBlocking,
}

impl<'a, T: FileDescriptorBased + Debug> FileLock<'a, T> {
    fn new(
        value: T,
        config: FileLockBuilder,
        handle: &'a ReadWriteMutexHandle<T>,
    ) -> Result<Self, ReadWriteMutexCreationError> {
        Ok(Self {
            file: fail!(from config, when ReadWriteMutexBuilder::new()
                .is_interprocess_capable(false)
                .clock_type(config.clock_type)
                .mutex_priority(config.priority)
                .create(value, handle),
                "Failed to create ReadWriteMutex for FileLock."),
            clock_type: config.clock_type,
            lock_state: AtomicI64::new(0),
        })
    }

    /// Blocking until the write lock of the underlying file is acquired. Returns a [`FileLockWriteGuard`]
    /// which provides read and write access to the underlying file and releases the lock as soon
    /// as it goes out of scope.
    /// A write-lock can be acquired when no reader and no writer locks are acquired by any
    /// other participant.
    pub fn write_lock(&self) -> Result<FileLockWriteGuard<'_, '_, T>, FileWriterGetLockError> {
        let guard = fail!(from self, when self.file.write_lock(),
            "Failed to acquire writer mutex lock in write_lock");
        self.internal_lock(
            LockType::Write,
            InternalMode::Blocking,
            guard.file_descriptor(),
        )?;

        Ok(FileLockWriteGuard {
            file_lock: self,
            guard,
        })
    }

    /// Tries to acquire the write lock in a non-blocking way. If the lock could be acquired it returns a [`FileLockWriteGuard`]
    /// which provides read and write access to the underlying file and releases the lock as soon
    /// as it goes out of scope. Otherwise it returns [`None`]
    /// A write-lock can be acquired when no reader and no writer locks are acquired by any
    /// other participant.
    pub fn write_try_lock(
        &self,
    ) -> Result<Option<FileLockWriteGuard<'_, '_, T>>, FileWriterTryLockError> {
        let guard = fail!(from self, when self.file.write_try_lock(),
                     "Failed while trying to acquire writer mutex lock in write_try_lock");

        if guard.is_none() {
            return Ok(None);
        }

        match self.internal_lock(
            LockType::Write,
            InternalMode::NonBlocking,
            guard.as_ref().unwrap().file_descriptor(),
        )? {
            true => Ok(Some(FileLockWriteGuard {
                file_lock: self,
                guard: guard.unwrap(),
            })),
            false => Ok(None),
        }
    }

    /// Tries to acquire the write lock and waits until timeout has passed. If the lock could be
    /// acquired it returns a [`FileLockWriteGuard`] which provides read and write access to the
    /// underlying file and releases the lock as soon as it goes out of scope. Otherwise it
    /// returns [`None`].
    /// A write-lock can be acquired when no reader and no writer locks are acquired by any
    /// other participant.
    pub fn write_timed_lock(
        &self,
        timeout: Duration,
    ) -> Result<Option<FileLockWriteGuard<'_, '_, T>>, FileWriterTimedLockError> {
        let time = fail!(from self, when Time::now_with_clock(self.clock_type),
                            "Failed to acquire current system time in write_timed_lock.");

        let guard = fail!(from self, when self.file.write_timed_lock(timeout),
                            "Failed while trying to acquire writer mutex lock in write_timed_lock");

        if guard.is_none() {
            return Ok(None);
        }

        let elapsed_time = fail!(from self, when time.elapsed(),
                            "Failed to acquire elapsed time in read_timed_lock.");

        if elapsed_time >= timeout {
            return Ok(None);
        }

        match self.internal_timed_lock(
            LockType::Write,
            timeout - elapsed_time,
            guard.as_ref().unwrap().file_descriptor(),
        )? {
            true => Ok(Some(FileLockWriteGuard {
                file_lock: self,
                guard: guard.unwrap(),
            })),
            false => Ok(None),
        }
    }

    /// Blocking until the read lock of the underlying file is acquired. Returns a
    /// [`FileLockReadGuard`] which provides read access to the underlying file and releases the
    /// lock as soon as it goes out of scope.
    /// A read-lock can be acquired when no write lock is acquired by any other participant.
    pub fn read_lock(&self) -> Result<FileLockReadGuard<'_, '_, T>, FileReaderGetLockError> {
        let guard = fail!(from self, when self.file.read_lock(),
                         "Failed to acquire reader mutex lock in read_lock");

        self.internal_lock(
            LockType::Read,
            InternalMode::Blocking,
            guard.file_descriptor(),
        )?;

        Ok(FileLockReadGuard {
            file_lock: self,
            guard,
        })
    }

    /// Tries to acquire a read lock of the underlying file. If the lock could be acquired it returns a
    /// [`FileLockReadGuard`] which provides read access to the underlying file and releases the
    /// lock as soon as it goes out of scope. Otherwise it returns [`None`].
    /// A read-lock can be acquired when no write lock is acquired by any other participant.
    pub fn read_try_lock(
        &self,
    ) -> Result<Option<FileLockReadGuard<'_, '_, T>>, FileReaderTryLockError> {
        let guard = fail!(from self, when self.file.read_try_lock(),
                            "Failed while trying to acquire reader mutex lock in read_try_lock");

        if guard.is_none() {
            return Ok(None);
        }

        match self.internal_lock(
            LockType::Read,
            InternalMode::NonBlocking,
            guard.as_ref().unwrap().file_descriptor(),
        )? {
            true => Ok(Some(FileLockReadGuard {
                file_lock: self,
                guard: guard.unwrap(),
            })),
            false => Ok(None),
        }
    }

    /// Tries to acquire a read lock and waits until timeout has passed. If the lock could be acquired it returns a
    /// [`FileLockReadGuard`] which provides read access to the underlying file and releases the
    /// lock as soon as it goes out of scope. Otherwise it returns [`None`].
    /// A read-lock can be acquired when no write lock is acquired by any other participant.
    pub fn read_timed_lock(
        &self,
        timeout: Duration,
    ) -> Result<Option<FileLockReadGuard<'_, '_, T>>, FileReaderTimedLockError> {
        let time = fail!(from self, when Time::now_with_clock(self.clock_type),
                         "Failed to acquire current system time in read_timed_lock.");

        let guard = fail!(from self, when self.file.read_timed_lock(timeout),
                         "Failed while trying to acquire writer mutex lock in read_timed_lock");

        if guard.is_none() {
            return Ok(None);
        }

        let elapsed_time = fail!(from self, when time.elapsed(),
                            "Failed to acquire elapsed time in read_timed_lock.");

        if elapsed_time >= timeout {
            return Ok(None);
        }

        match self.internal_timed_lock(
            LockType::Read,
            timeout - elapsed_time,
            guard.as_ref().unwrap().file_descriptor(),
        )? {
            true => Ok(Some(FileLockReadGuard {
                file_lock: self,
                guard: guard.unwrap(),
            })),
            false => Ok(None),
        }
    }

    /// Returns the current [`LockState`] of the [`FileLock`].
    pub fn get_lock_state(&self) -> Result<LockState, FileLockStateError> {
        match 0.cmp(&self.lock_state.load(Ordering::Relaxed)) {
            std::cmp::Ordering::Less => {
                return Ok(LockState {
                    lock_type: LockType::Read,
                    pid: Process::from_self().id(),
                })
            }
            std::cmp::Ordering::Greater => {
                return Ok(LockState {
                    lock_type: LockType::Write,
                    pid: Process::from_self().id(),
                })
            }
            std::cmp::Ordering::Equal => (),
        }

        let msg = "Unable to acquire current file lock state";
        let mut current_lock_state = posix::flock::new();
        current_lock_state.l_type = posix::F_WRLCK as _;

        let fd_guard = fail!(from self, when self.file.read_lock(),
            "{} due to an internal failure in while acquiring the mutex.", msg);

        match unsafe {
            posix::fcntl(
                fd_guard.file_descriptor().native_handle(),
                posix::F_GETLK,
                &mut current_lock_state,
            )
        } != -1
        {
            true => Ok(LockState {
                lock_type: match current_lock_state.l_type as i32 {
                    posix::F_WRLCK => LockType::Write,
                    posix::F_RDLCK => LockType::Read,
                    _ => LockType::Unlock,
                },
                pid: ProcessId::new(current_lock_state.l_pid),
            }),
            false => handle_errno!(FileLockStateError, from self,
                Errno::EBADF => (InvalidFileDescriptor, "{} since the file-descriptor is invalid or not opened in the correct mode.", msg),
                Errno::EINTR => (Interrupt, "{} since an interrupt signal was received.", msg),
                v => (UnknownError(v as i32), "{} since an unknown error occurred ({}).", msg, v)
            ),
        }
    }

    fn release(&self, file_descriptor: &FileDescriptor) -> Result<(), FileUnlockError> {
        let mut new_lock_state = posix::flock::new();
        new_lock_state.l_type = LockType::Unlock as _;
        new_lock_state.l_whence = posix::SEEK_SET as _;

        let msg = "Unable to release file-lock";
        if unsafe {
            posix::fcntl(
                file_descriptor.native_handle(),
                posix::F_SETLK,
                &mut new_lock_state,
            )
        } != -1
        {
            self.set_lock_state(LockType::Unlock);
            return Ok(());
        }

        handle_errno!(FileUnlockError, from self,
            Errno::EBADF => (InvalidFileDescriptorOrWrongOpenMode, "{} since the file-descriptor is invalid or not opened in the correct mode.", msg),
            Errno::EINTR => (Interrupt, "{} since an interrupt signal was received.", msg),
            v => (UnknownError(v as i32), "{} since an unknown error occurred ({}).", msg, v)
        );
    }

    fn internal_timed_lock(
        &self,
        lock_type: LockType,
        timeout: Duration,
        file_descriptor: &FileDescriptor,
    ) -> Result<bool, FileTimedLockError> {
        let msg = "Unable to wait in timed_lock with timeout ".to_string()
            + &timeout.as_secs_f64().to_string()
            + "s";
        let time = fail!(from self, when Time::now_with_clock(self.clock_type), "Failed to acquire current system time in timed_lock.");
        let mut adaptive_wait = fail!(from self, when AdaptiveWaitBuilder::new()
            .clock_type(self.clock_type)
            .create(), "{} since the adaptive wait could not be created.", msg);

        loop {
            match self.internal_lock(lock_type, InternalMode::NonBlocking, file_descriptor)? {
                true => {
                    return Ok(true);
                }
                false => {
                    match fail!(from self, when time.elapsed(),
                        "Failed to acquire elapsed time in timed_lock.")
                        < timeout
                    {
                        true => {
                            fail!(from self, when adaptive_wait.wait(),
                                "AdaptiveWait failed in timed_lock.");
                        }
                        false => return Ok(false),
                    }
                }
            }
        }
    }

    fn internal_lock(
        &self,
        lock_type: LockType,
        mode: InternalMode,
        file_descriptor: &FileDescriptor,
    ) -> Result<bool, FileTryLockError> {
        let mut new_lock_state = posix::flock::new();
        new_lock_state.l_type = lock_type as _;
        new_lock_state.l_whence = posix::SEEK_SET as _;

        if unsafe {
            posix::fcntl(
                file_descriptor.native_handle(),
                if mode == InternalMode::NonBlocking {
                    posix::F_SETLK
                } else {
                    posix::F_SETLKW
                },
                &mut new_lock_state,
            )
        } != -1
        {
            self.set_lock_state(lock_type);
            return Ok(true);
        }

        let msg = match lock_type {
            LockType::Read => "Unable to acquire read file-lock",
            _ => "Unable to acquire write file-lock",
        };

        handle_errno!(FileTryLockError, from self,
            success Errno::EACCES => false;
            success Errno::EAGAIN => false,
            Errno::EBADF => (InvalidFileDescriptorOrWrongOpenMode, "{} since the file-descriptor is invalid or not opened in the correct mode.", msg),
            Errno::EINTR => (Interrupt, "{} since an interrupt signal was received.", msg),
            Errno::ENOLCK => (ExceedsMaximumNumberOfLockedRegionsInSystem, "{} since it would exceed the maximum supported number of locked regions in the system..", msg),
            Errno::EDEADLK => (DeadlockConditionDetected, "{} since a deadlock condition was detected.", msg),
            v => (UnknownError(v as i32), "{} since an unknown error occurred ({}).", msg, v)
        );
    }

    fn set_lock_state(&self, value: LockType) {
        let current_value = self.lock_state.load(Ordering::Relaxed);
        let adjustment = match value {
            LockType::Read => 1,
            LockType::Write => -1,
            LockType::Unlock => {
                if current_value > 0 {
                    -1
                } else {
                    1
                }
            }
        };

        self.lock_state.fetch_add(adjustment, Ordering::Relaxed);
    }
}
