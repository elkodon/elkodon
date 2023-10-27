//! Provides the trait [`GroupExt`] to create groups from strings by interpreting them as group
//! name or from unsigned integers by interpreting them as group id. The [`Group`] struct provides
//! access to the properties of a POSIX group.
//!
//! # Example
//!
//! ## Working with groups
//!
//! ```
//! use elkodon_bb_posix::group::*;
//! use elkodon_bb_system_types::group_name::GroupName;
//! use elkodon_bb_container::semantic_string::*;
//!
//! let myself = Group::from_self().expect("failed to get group");
//! let root = Group::from_name(&GroupName::new(b"root").unwrap())
//!                     .expect("failed to get root group");
//!
//! println!("I am in group {:?} and the root group is {:?}", myself, root);
//!
//! println!("Members of my group:");
//! for member in myself.members() {
//!     println!("{}", member);
//! }
//! ```
//!
//! ## Use the trait
//!
//! ```
//! use elkodon_bb_posix::group::*;
//!
//! println!("Members of group root");
//! for member in "root".as_group().unwrap().members() {
//!     println!("{}", member);
//! }
//! ```

use std::ffi::CStr;

use elkodon_bb_container::byte_string::strlen;
use elkodon_bb_container::semantic_string::*;
use elkodon_bb_elementary::enum_gen;
use elkodon_bb_system_types::{group_name::GroupName, user_name::UserName};
use elkodon_pal_posix::posix::errno::Errno;
use elkodon_pal_posix::posix::Struct;
use elkodon_pal_posix::*;

use crate::{config::GROUP_BUFFER_SIZE, system_configuration::*};
use elkodon_bb_log::fail;

enum_gen! { GroupError
  entry:
    Interrupt,
    IOerror,
    PerProcessFileHandleLimitReached,
    SystemWideFileHandleLimitReached,
    InsufficientBufferSize,
    GroupNotFound,
    SystemGroupNameLengthLongerThanSupportedLength,
    SystemUserNameLengthLongerThanSupportedLength,
    InvalidGroupName,
    UnknownError(i32)
}

/// Trait to create a [`Group`] from an integer by interpreting it as the gid or from a [`String`]
/// or [`str`] by interpreting the value as group name.
pub trait GroupExt {
    fn as_group(&self) -> Result<Group, GroupError>;
}

impl GroupExt for u32 {
    fn as_group(&self) -> Result<Group, GroupError> {
        Group::from_gid(*self)
    }
}

impl GroupExt for String {
    fn as_group(&self) -> Result<Group, GroupError> {
        Group::from_name(
            &fail!(from "String::as_group()", when GroupName::new(self.as_bytes()),
                        with GroupError::InvalidGroupName,
                        "Failed to create group object since the name \"{}\" contains invalid characters.",
                        self),
        )
    }
}

impl GroupExt for &str {
    fn as_group(&self) -> Result<Group, GroupError> {
        Group::from_name(
            &fail!(from "&str::as_group()", when GroupName::new(self.as_bytes()),
                        with GroupError::InvalidGroupName,
                        "Failed to create group object since the name \"{}\" contains invalid characters.",
                        self),
        )
    }
}

impl GroupExt for GroupName {
    fn as_group(&self) -> Result<Group, GroupError> {
        Group::from_name(self)
    }
}

/// Represents a group in a POSIX system
#[derive(Debug)]
pub struct Group {
    gid: u32,
    name: GroupName,
    password: String,
    members: Vec<UserName>,
}

enum Source {
    Gid,
    GroupName,
}

impl Group {
    /// Create an group object from the owners group of the process
    pub fn from_self() -> Result<Group, GroupError> {
        Self::from_gid(unsafe { posix::getgid() })
    }

    /// Create an group object from a given gid. If the gid does not exist an error will be
    /// returned.
    pub fn from_gid(gid: u32) -> Result<Group, GroupError> {
        let mut new_group = Group {
            gid,
            name: unsafe { GroupName::new_empty() },
            password: String::new(),
            members: vec![],
        };

        new_group.populate_entries(Source::Gid)?;

        Ok(new_group)
    }

    /// Create an group object from a given group-name. If the group-name does not exist an error will
    /// be returned
    pub fn from_name(group_name: &GroupName) -> Result<Group, GroupError> {
        let mut new_group = Group {
            gid: u32::MAX,
            name: *group_name,
            password: String::new(),
            members: vec![],
        };

        new_group.populate_entries(Source::GroupName)?;

        Ok(new_group)
    }

    /// Return the group id
    pub fn gid(&self) -> u32 {
        self.gid
    }

    /// Return the group name
    pub fn name(&self) -> &GroupName {
        &self.name
    }

    /// Old entry, should contain only 'x'. Returns the password of the group but on modern systems
    /// it should be stored in /etc/shadow
    pub fn password(&self) -> &str {
        self.password.as_str()
    }

    /// Returns a list of all the group members as string
    pub fn members(&self) -> Vec<UserName> {
        self.members.clone()
    }

    fn extract_entry(&self, field: *mut posix::char, name: &str) -> Result<String, GroupError> {
        Ok(
            fail!(from self, when unsafe { CStr::from_ptr(field) }.to_str(),
                with GroupError::InvalidGroupName,
                "The {} contains invalid UTF-8 symbols.", name)
            .to_string(),
        )
    }

    fn populate_entries(&mut self, source: Source) -> Result<(), GroupError> {
        let mut group = posix::group::new();
        let mut group_ptr: *mut posix::group = &mut group;
        let mut buffer: [posix::char; GROUP_BUFFER_SIZE] = [0; GROUP_BUFFER_SIZE];

        let msg;
        let errno_value = match source {
            Source::GroupName => {
                msg = "Unable to acquire group entry from groupname";
                unsafe {
                    posix::getgrnam_r(
                        self.name.as_c_str(),
                        &mut group,
                        buffer.as_mut_ptr(),
                        GROUP_BUFFER_SIZE,
                        &mut group_ptr,
                    )
                }
            }
            Source::Gid => {
                msg = "Unable to acquire group entry from gid";
                unsafe {
                    posix::getgrgid_r(
                        self.gid,
                        &mut group,
                        buffer.as_mut_ptr(),
                        GROUP_BUFFER_SIZE,
                        &mut group_ptr,
                    )
                }
            }
        }
        .into();

        handle_errno!(GroupError, from self,
            errno_source errno_value, continue_on_success,
            success Errno::ESUCCES => (),
            Errno::EINTR => (Interrupt, "{} since an interrupt signal was received", msg ),
            Errno::EIO => (IOerror, "{} due to an I/O error.", msg),
            Errno::EMFILE => (PerProcessFileHandleLimitReached, "{} since the per-process file handle limit is reached.", msg ),
            Errno::ENFILE => (SystemWideFileHandleLimitReached, "{} since the system-wide file handle limit is reached.", msg),
            Errno::ERANGE => (InsufficientBufferSize, "{} since insufficient storage was provided. Max buffer size should be: {}", msg, Limit::MaxSizeOfPasswordBuffer.value()),
            v => (UnknownError(v as i32), "{} due to an unknown error ({}).", msg, v)
        );

        if group_ptr.is_null() {
            fail!(from self, with GroupError::GroupNotFound, "{} since the group does not exist.", msg);
        }

        self.gid = group.gr_gid;
        self.name = fail!(from self, when unsafe{ GroupName::from_c_str(group.gr_name) },
                            with GroupError::SystemGroupNameLengthLongerThanSupportedLength,
                            "{} since the group name length ({}) is greater than the supported group name length of {}.",
                            msg, unsafe { strlen(group.gr_name) }, GroupName::max_len() );
        self.password = self.extract_entry(group.gr_passwd, "password")?;

        let mut counter: isize = 0;
        loop {
            let group_member = unsafe { *group.gr_mem.offset(counter) };
            if group_member.is_null() {
                break;
            }

            self.members
                .push(fail!(from self, when unsafe { UserName::from_c_str(group_member) },
                        with GroupError::SystemUserNameLengthLongerThanSupportedLength,
                        "{} since the user name length ({}) is greater than the support user name length of {}.",
                        msg, unsafe { strlen(group_member) }, UserName::max_len() ));
            counter += 1;
        }

        Ok(())
    }
}
