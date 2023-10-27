#![allow(non_camel_case_types)]
#![allow(clippy::missing_safety_doc)]
#![allow(unused_variables)]

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::{
    posix::CLOCK_REALTIME,
    posix::{types::*, Errno},
};

pub unsafe fn clock_gettime(clock_id: clockid_t, tp: *mut timespec) -> int {
    if clock_id != CLOCK_REALTIME {
        return Errno::EINVAL as _;
    }

    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Err(_) => Errno::EINVAL as _,
        Ok(v) => {
            (*tp).tv_sec = v.as_secs() as _;
            (*tp).tv_nsec = v.subsec_nanos() as _;
            Errno::ESUCCES as _
        }
    }
}

pub unsafe fn clock_settime(clock_id: clockid_t, tp: *const timespec) -> int {
    if clock_id != CLOCK_REALTIME {
        return Errno::EINVAL as _;
    }

    Errno::ENOSYS as _
}

pub unsafe fn clock_nanosleep(
    clock_id: clockid_t,
    flags: int,
    rqtp: *const timespec,
    rmtp: *mut timespec,
) -> int {
    if clock_id != CLOCK_REALTIME {
        return Errno::EINVAL as _;
    }

    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    if now.is_err() {
        return Errno::EINVAL as _;
    }

    let time = Duration::from_secs((*rqtp).tv_sec as _)
        + Duration::from_nanos((*rqtp).tv_nsec as _)
        - now.unwrap();

    std::thread::sleep(time);
    Errno::ESUCCES as _
}
