//! Contains all macros to log messages.

/// Logs a trace message.
///
/// ```
/// use elkodon_bb_log::trace;
///
/// #[derive(Debug)]
/// struct MyDataType {}
///
/// impl MyDataType {
///     fn something_that_fails(&self) -> Result<(), ()> {
///         Err(())
///     }
///
///     fn doIt(&self) {
///         trace!("Only a message");
///         trace!(from self, "Message which adds the object as its origin");
///         trace!(from "Somewhere over the Rainbow", "Message with custom origin");
///
///         trace!(from self, when self.something_that_fails(),
///             "Print only when result.is_err()")
///     }
/// }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! trace {
    ($($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Trace, std::format_args!(""), std::format_args!($($e),*))
    };
    (from $o:expr, $($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Trace, std::format_args!("{:?}", $o), std::format_args!($($e),*))
    };
    (from $o:expr, when $call:expr, $($e:expr),*) => {
        {
            let result = $call;
            if result.is_err() {
                $crate::__internal_print_log_msg($crate::LogLevel::Trace, std::format_args!("{:?}", $o), std::format_args!($($e),*))
            }
            result.ok().unwrap()
        }
    }
}

/// Logs a debug message.
///
/// ```
/// use elkodon_bb_log::debug;
///
/// #[derive(Debug)]
/// struct MyDataType {}
///
/// impl MyDataType {
///     fn something_that_fails(&self) -> Result<(), ()> {
///         Err(())
///     }
///
///     fn doIt(&self) {
///         debug!("Only a message");
///         debug!(from self, "Message which adds the object as its origin");
///         debug!(from "Somewhere over the Rainbow", "Message with custom origin");
///
///         debug!(from self, when self.something_that_fails(),
///             "Print only when result.is_err()")
///     }
/// }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! debug {
    ($($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Debug, std::format_args!(""), std::format_args!($($e),*))
    };
    (from $o:expr, $($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Debug, std::format_args!("{:?}", $o), std::format_args!($($e),*))
    };
    (from $o:expr, when $call:expr, $($e:expr),*) => {
        {
            let result = $call;
            if result.is_err() {
                $crate::__internal_print_log_msg($crate::LogLevel::Debug, std::format_args!("{:?}", $o), std::format_args!($($e),*))
            }
            result.ok().unwrap()
        }
    }
}

/// Logs a info message.
///
/// ```
/// use elkodon_bb_log::info;
///
/// #[derive(Debug)]
/// struct MyDataType {}
///
/// impl MyDataType {
///     fn something_that_fails(&self) -> Result<(), ()> {
///         Err(())
///     }
///
///     fn doIt(&self) {
///         info!("Only a message");
///         info!(from self, "Message which adds the object as its origin");
///         info!(from "Somewhere over the Rainbow", "Message with custom origin");
///
///         info!(from self, when self.something_that_fails(),
///             "Print only when result.is_err()")
///     }
/// }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! info {
    ($($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Info, std::format_args!(""), std::format_args!($($e),*))
    };
    (from $o:expr, $($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Info, std::format_args!("{:?}", $o), std::format_args!($($e),*))
    };
    (from $o:expr, when $call:expr, $($e:expr),*) => {
        {
            let result = $call;
            if result.is_err() {
                $crate::__internal_print_log_msg($crate::LogLevel::Info, std::format_args!("{:?}", $o), std::format_args!($($e),*))
            }
            result.ok().unwrap()
        }
    }
}

/// Logs a warn message.
///
/// ```
/// use elkodon_bb_log::warn;
///
/// #[derive(Debug)]
/// struct MyDataType {}
///
/// impl MyDataType {
///     fn something_that_fails(&self) -> Result<(), ()> {
///         Err(())
///     }
///
///     fn doIt(&self) {
///         warn!("Only a message");
///         warn!(from self, "Message which adds the object as its origin");
///         warn!(from "Somewhere over the Rainbow", "Message with custom origin");
///
///         warn!(from self, when self.something_that_fails(),
///             "Print only when result.is_err()")
///     }
/// }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! warn {
    ($($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Warn, std::format_args!(""), std::format_args!($($e),*))
    };
    (from $o:expr, $($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Warn, std::format_args!("{:?}", $o), std::format_args!($($e),*))
    };
    (from $o:expr, when $call:expr, $($e:expr),*) => {
        {
            let result = $call;
            if result.is_err() {
                $crate::__internal_print_log_msg($crate::LogLevel::Warn, std::format_args!("{:?}", $o), std::format_args!($($e),*))
            }
            result.ok().unwrap()
        }
    }
}

/// Logs an error message.
///
/// ```
/// use elkodon_bb_log::error;
///
/// #[derive(Debug)]
/// struct MyDataType {}
///
/// impl MyDataType {
///     fn something_that_fails(&self) -> Result<(), ()> {
///         Err(())
///     }
///
///     fn doIt(&self) {
///         error!("Only a message");
///         error!(from self, "Message which adds the object as its origin");
///         error!(from "Somewhere over the Rainbow", "Message with custom origin");
///
///         error!(from self, when self.something_that_fails(),
///             "Print only when result.is_err()")
///     }
/// }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! error {
    ($($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Error, std::format_args!(""), std::format_args!($($e),*))
    };
    (from $o:expr, $($e:expr),*) => {
        $crate::__internal_print_log_msg($crate::LogLevel::Error, std::format_args!("{:?}", $o), std::format_args!($($e),*))
    };
    (from $o:expr, when $call:expr, $($e:expr),*) => {
        {
            let result = $call;
            if result.is_err() {
                $crate::__internal_print_log_msg($crate::LogLevel::Error, std::format_args!("{:?}", $o), std::format_args!($($e),*))
            }
            result.ok().unwrap()
        }
    }
}

/// Logs a fatal error message and calls panic.
///
/// ```
/// use elkodon_bb_log::fatal_panic;
///
/// #[derive(Debug)]
/// struct MyDataType {}
///
/// impl MyDataType {
///     fn something_that_fails(&self) -> Result<(), ()> {
///         Err(())
///     }
///
///     fn doIt(&self) {
///         fatal_panic!("Only a message");
///         fatal_panic!(from self, "Message which adds the object as its origin");
///         fatal_panic!(from "Somewhere over the Rainbow", "Message with custom origin");
///
///         fatal_panic!(from self, when self.something_that_fails(),
///             "Print only when result.is_err()")
///     }
/// }
/// ```
#[macro_export(local_inner_macros)]
macro_rules! fatal_panic {
    ($($e:expr),*) => {
        {
            $crate::__internal_print_log_msg($crate::LogLevel::Fatal, std::format_args!(""), std::format_args!($($e),*));
            std::panic!($($e),*);
        }
    };
    (from $o:expr, $($e:expr),*) => {
        {
            $crate::__internal_print_log_msg($crate::LogLevel::Fatal, std::format_args!("{:?}", $o), std::format_args!($($e),*));
            std::panic!("From: {:?} ::: {}", $o, std::format_args!($($e),*));
        }
    };
    (from $o:expr, when $call:expr, $($e:expr),*) => {
        {
            let result = $call;
            if result.is_err() {
                $crate::__internal_print_log_msg($crate::LogLevel::Fatal, std::format_args!("{:?}", $o), std::format_args!($($e),*));
                std::panic!("From: {:?} ::: {}", $o, std::format_args!($($e),*));
            }
            result.ok().unwrap()
        }
    }
}
