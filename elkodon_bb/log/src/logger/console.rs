//! The default [`Logger`] implementation.

use std::sync::atomic::{AtomicU64, Ordering};

use termsize::Size;

use crate::{get_log_level, LogLevel};

pub enum ConsoleLogOrder {
    Time,
    Counter,
}

pub struct Logger {
    counter: AtomicU64,
    ordering_mode: ConsoleLogOrder,
}

impl Logger {
    pub const fn new() -> Self {
        Self {
            counter: AtomicU64::new(0),
            ordering_mode: ConsoleLogOrder::Counter,
        }
    }

    fn log_level_string(log_level: crate::LogLevel) -> &'static str {
        match log_level {
            LogLevel::Trace => "\x1b[0;90m[T]",
            LogLevel::Debug => "\x1b[0;90m[D]",
            LogLevel::Info => "\x1b[0;92m[I]",
            LogLevel::Warn => "\x1b[0;33m[W]",
            LogLevel::Error => "\x1b[0;31m[E]",
            LogLevel::Fatal => "\x1b[1;4;91m[F]",
        }
    }

    fn message_color(log_level: crate::LogLevel) -> &'static str {
        match log_level {
            LogLevel::Trace => "\x1b[1;90m",
            LogLevel::Debug => "\x1b[1;90m",
            LogLevel::Info => "\x1b[1;37m",
            LogLevel::Warn => "\x1b[1;93m",
            LogLevel::Error => "\x1b[1;91m",
            LogLevel::Fatal => "\x1b[1;4;91m",
        }
    }

    fn counter_color(_log_level: crate::LogLevel) -> &'static str {
        "\x1b[0;90m"
    }

    fn origin_color(log_level: crate::LogLevel) -> &'static str {
        match log_level {
            LogLevel::Trace => "\x1b[0;90m",
            LogLevel::Debug => "\x1b[0;90m",
            LogLevel::Info => "\x1b[0;37m",
            LogLevel::Warn => "\x1b[0;37m",
            LogLevel::Error => "\x1b[0;37m",
            LogLevel::Fatal => "\x1b[0;4;91m",
        }
    }

    fn add_spacing(spacing: usize) {
        for _ in 0..spacing {
            std::print!(" ");
        }
    }

    fn print(first_spacing: usize, spacing: usize, separator: &str, color: &str, output: &str) {
        let term_len = Self::get_terminal_size().cols as usize - spacing - separator.len();
        let mut msg_len = output.len();
        let mut msg_pos = 0;

        Self::add_spacing(first_spacing);
        loop {
            std::print!("{}", color);
            if msg_len < term_len {
                std::print!("{}{}", separator, unsafe {
                    output.get_unchecked(msg_pos..msg_pos + msg_len)
                });
                break;
            } else {
                std::print!("{}{}", separator, unsafe {
                    output.get_unchecked(msg_pos..msg_pos + term_len)
                });
                msg_pos += term_len;
                msg_len -= term_len;
            }
            std::println!("\x1b[0m");
            Self::add_spacing(spacing);
        }

        println!("\x1b[0m");
    }

    fn print_message(spacing: usize, log_level: crate::LogLevel, formatted_message: &str) {
        Self::print(
            spacing,
            spacing,
            "| ",
            Self::message_color(log_level),
            formatted_message,
        );
    }

    fn print_origin(spacing: usize, log_level: crate::LogLevel, origin: &str) {
        print!("{} ", Logger::log_level_string(log_level));
        Self::print(0, spacing, "", Logger::origin_color(log_level), origin);
    }

    fn get_terminal_size() -> Size {
        match termsize::get() {
            None => Size {
                rows: 9999,
                cols: 9999,
            },
            Some(t) => t,
        }
    }
}

impl crate::logger::Logger for Logger {
    fn log(
        &self,
        log_level: crate::LogLevel,
        origin: std::fmt::Arguments,
        formatted_message: std::fmt::Arguments,
    ) {
        let counter = self.counter.fetch_add(1, Ordering::Relaxed);

        if get_log_level() > log_level as u8 {
            return;
        }

        let origin_str = origin.to_string();
        let msg_str = formatted_message.to_string();

        let mut spacing = 0;
        match self.ordering_mode {
            ConsoleLogOrder::Time => {
                let time = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap();

                match origin_str.is_empty() {
                    false => {
                        std::print!(
                            "{}{}.{:0>9} ",
                            Logger::counter_color(log_level),
                            time.as_secs(),
                            time.subsec_nanos(),
                        );
                        spacing = 25;
                        Self::print_origin(spacing, log_level, &origin_str);
                    }
                    true => std::println!(
                        "{}{}.{:0>9} {} ",
                        Logger::counter_color(log_level),
                        time.as_secs(),
                        time.subsec_nanos(),
                        Logger::log_level_string(log_level),
                    ),
                }
            }
            ConsoleLogOrder::Counter => match origin.to_string().is_empty() {
                false => {
                    std::print!("{}{:9} ", Logger::counter_color(log_level), counter);
                    spacing = 14;
                    Self::print_origin(spacing, log_level, &origin_str);
                }
                true => std::print!(
                    "{}{:9} {} ",
                    Logger::counter_color(log_level),
                    counter,
                    Logger::log_level_string(log_level),
                ),
            },
        }

        Self::print_message(spacing, log_level, &msg_str);
    }
}
