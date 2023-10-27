#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(unused_variables)]

use elkodon_pal_concurrency_primitives::mutex::Mutex;
use windows_sys::Win32::{
    Foundation::{FALSE, TRUE},
    System::{
        Console::{
            GenerateConsoleCtrlEvent, SetConsoleCtrlHandler, CTRL_BREAK_EVENT, CTRL_CLOSE_EVENT,
            CTRL_C_EVENT,
        },
        Threading::{GetExitCodeProcess, OpenProcess, PROCESS_ALL_ACCESS},
    },
};

use core::cell::UnsafeCell;

use crate::{
    posix::getpid,
    posix::types::*,
    posix::{Errno, SIGKILL, SIGSTOP, SIGTERM, SIGUSR1},
    win32call,
};

struct SigAction {
    action: UnsafeCell<sigaction_t>,
    mtx: Mutex,
}

impl SigAction {
    const fn new() -> Self {
        Self {
            action: UnsafeCell::new(sigaction_t {
                sa_handler: 0,
                sa_mask: sigset_t {},
                sa_flags: 0,
                sa_restorer: None,
            }),
            mtx: Mutex::new(),
        }
    }

    fn get(&self) -> sigaction_t {
        self.mtx.lock(|_, _| true);
        let ret_val = unsafe { *self.action.get() };
        self.mtx.unlock(|_| {});
        ret_val
    }

    fn set(&self, value: sigaction_t) -> sigaction_t {
        self.mtx.lock(|_, _| true);
        let ret_val = unsafe { *self.action.get() };
        unsafe { *self.action.get() = value };
        self.mtx.unlock(|_| {});
        ret_val
    }
}

unsafe impl Send for SigAction {}
unsafe impl Sync for SigAction {}

static SIG_ACTION: SigAction = SigAction::new();

unsafe extern "system" fn ctrl_handler(value: u32) -> i32 {
    let action =
        core::mem::transmute::<sighandler_t, extern "C" fn(int)>(SIG_ACTION.get().sa_handler);

    let sigval = win32_event_to_signal(value);

    action(sigval);
    0
}

fn signal_to_win32_event(sig: int) -> Option<u32> {
    match sig {
        SIGTERM => Some(CTRL_C_EVENT),
        SIGSTOP => Some(CTRL_BREAK_EVENT),
        SIGKILL => Some(CTRL_CLOSE_EVENT),
        _ => None,
    }
}

fn win32_event_to_signal(event: u32) -> int {
    match event {
        CTRL_C_EVENT => SIGTERM,
        CTRL_BREAK_EVENT => SIGSTOP,
        CTRL_CLOSE_EVENT => SIGKILL,
        _ => SIGUSR1,
    }
}

pub unsafe fn sigaction(sig: int, act: *const sigaction_t, oact: *mut sigaction_t) -> int {
    (*oact) = SIG_ACTION.set(*act);

    if (*act).sa_handler == 0 {
        SetConsoleCtrlHandler(Some(ctrl_handler), TRUE);
    } else {
        SetConsoleCtrlHandler(None, FALSE);
    }

    0
}

pub unsafe fn kill(pid: pid_t, sig: int) -> int {
    if sig == 0 {
        let mut exit_code = 0;
        let handle = win32call! { OpenProcess(PROCESS_ALL_ACCESS, TRUE, pid) };
        return if win32call! { GetExitCodeProcess(handle, &mut exit_code) } == TRUE {
            0
        } else {
            Errno::set(Errno::ESRCH);
            -1
        };
    }

    if pid != getpid() {
        Errno::set(Errno::ENOTSUP);
        return -1;
    }

    match signal_to_win32_event(sig) {
        None => {
            Errno::set(Errno::ENOTSUP);
            -1
        }
        Some(e) => {
            win32call! {GenerateConsoleCtrlEvent(e, 0)};
            0
        }
    }
}
