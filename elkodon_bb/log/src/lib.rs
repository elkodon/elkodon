//! Simplistic logger. It has 6 [`LogLevel`]s which can be set via [`set_log_level()`] and read via
//! [`get_log_level()`].
//!
//! The logger provides convinience macros to combine error/panic handling directly with the
//! logger.
//! The [`fail!`] macro can return when the function which was called return an error containing
//! result.
//! The [`fatal_panic!`] macro calls [`panic!`].
//!
//! # Example
//!
//! ## Logging
//! ```
//! use elkodon_bb_log::{debug, error, info, trace, warn};
//!
//! #[derive(Debug)]
//! struct MyDataType {
//!     value: u64
//! }
//!
//! impl MyDataType {
//!     fn log_stuff(&self) {
//!         trace!("trace message");
//!         trace!(from self, "trace message");
//!         trace!(from "Custom::Origin", "trace message");
//!
//!         debug!("hello {} {}", 123, 456);
//!         debug!(from self, "hello {}", 123);
//!         debug!(from "Another::Origin", "hello {}", 123);
//!
//!         info!("world");
//!         info!(from self, "world");
//!         info!(from "hello", "world");
//!
//!         warn!("warn message");
//!         warn!(from self, "warning");
//!         warn!(from "Somewhere::Else", "warning!");
//!
//!         error!("bla {}", 1);
//!         error!(from self, "bla {}", 1);
//!         error!(from "error origin", "bla {}", 1);
//!     }
//!}
//! ```
//!
//! ## Error Handling
//! ```
//! use elkodon_bb_log::fail;
//!
//! #[derive(Debug)]
//! struct MyDataType {
//!     value: u64
//! }
//!
//! impl MyDataType {
//!     fn doStuff(&self, value: u64) -> Result<(), ()> {
//!         if value == 0 { Err(()) } else { Ok(()) }
//!     }
//!
//!     fn doMoreStuff(&self) -> Result<(), u64> {
//!         // fail when doStuff.is_err() and return the error 1234
//!         fail!(from self, when self.doStuff(0),
//!                 with 1234, "Failed while calling doStuff");
//!         Ok(())
//!     }
//!
//!     fn doMore(&self) -> Result<(), u64> {
//!         if self.value == 0 {
//!             // without condition, return error 4567
//!             fail!(from self, with 4567, "Value is zero");
//!         }
//!
//!         Ok(())
//!     }
//!
//!     fn evenMore(&self) -> Result<(), u64> {
//!         // forward error when it is compatible or convertable
//!         fail!(from self, when self.doMore(), "doMore failed");
//!         Ok(())
//!     }
//! }
//! ```
//!
//! ## Panic Handling
//! ```
//! use elkodon_bb_log::fatal_panic;
//!
//! #[derive(Debug)]
//! struct MyDataType {
//!     value: u64
//! }
//!
//! impl MyDataType {
//!     fn doStuff(&self, value: u64) {
//!         if value == 0 {
//!             fatal_panic!(from self, "value is {}", value);
//!         }
//!     }
//!
//!     fn moreStuff(&self) -> Result<(), ()> {
//!         if self.value == 0 { Err(()) } else { Ok(()) }
//!     }
//!
//!     fn doIt(&self) {
//!         fatal_panic!(from self, when self.moreStuff(), "moreStuff failed");
//!     }
//! }
//! ```
//!

#[macro_use]
pub mod log;
#[macro_use]
pub mod fail;
pub mod logger;

use std::{
    fmt::Arguments,
    sync::{
        atomic::{AtomicU8, Ordering},
        Once,
    },
};

use logger::Logger;

static DEFAULT_LOGGER: logger::console::Logger = logger::console::Logger::new();
static mut LOGGER: Option<&'static dyn logger::Logger> = None;
static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::Trace as u8);
static INIT: Once = Once::new();

/// Describes the log level.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum LogLevel {
    Trace = 0,
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4,
    Fatal = 5,
}

/// Sets the current log level
pub fn set_log_level(v: LogLevel) {
    LOG_LEVEL.store(v as u8, Ordering::Relaxed);
}

/// Returns the current log level
pub fn get_log_level() -> u8 {
    LOG_LEVEL.load(Ordering::Relaxed)
}

/// Sets the [`Logger`]. Can be only called once at the beginning of the program. If the
/// [`Logger`] is already set it returns false and does not update it.
pub fn set_logger<T: logger::Logger + 'static>(value: &'static T) -> bool {
    let mut set_logger_success = false;
    INIT.call_once(|| {
        unsafe { LOGGER = Some(value) };
        set_logger_success = true;
    });

    set_logger_success
}

/// Returns a reference to the [`Logger`].
pub fn get_logger() -> &'static dyn Logger {
    INIT.call_once(|| {
        unsafe { LOGGER = Some(&DEFAULT_LOGGER) };
    });

    unsafe { *LOGGER.as_ref().unwrap() }
}

#[doc(hidden)]
pub fn __internal_print_log_msg(log_level: LogLevel, origin: Arguments, args: Arguments) {
    get_logger().log(log_level, origin, args)
}
