#![allow(non_camel_case_types, non_snake_case)]
#![allow(clippy::missing_safety_doc)]

use crate::posix::types::*;

pub unsafe fn select(
    nfds: int,
    readfds: *mut fd_set,
    writefds: *mut fd_set,
    errorfds: *mut fd_set,
    timeout: *mut timeval,
) -> int {
    crate::internal::select(nfds, readfds, writefds, errorfds, timeout)
}

pub unsafe fn CMSG_SPACE(length: size_t) -> size_t {
    internal::elkodon_cmsg_space(length)
}

pub unsafe fn CMSG_FIRSTHDR(mhdr: *const msghdr) -> *mut cmsghdr {
    internal::elkodon_cmsg_firsthdr(mhdr)
}

pub unsafe fn CMSG_NXTHDR(header: *const msghdr, sub_header: *const cmsghdr) -> *mut cmsghdr {
    internal::elkodon_cmsg_nxthdr(header as *mut msghdr, sub_header as *mut cmsghdr)
}

pub unsafe fn CMSG_LEN(length: size_t) -> size_t {
    internal::elkodon_cmsg_len(length)
}

pub unsafe fn CMSG_DATA(cmsg: *mut cmsghdr) -> *mut uchar {
    internal::elkodon_cmsg_data(cmsg)
}

pub unsafe fn FD_CLR(fd: int, set: *mut fd_set) {
    internal::elkodon_fd_clr(fd, set)
}

pub unsafe fn FD_ISSET(fd: int, set: *const fd_set) -> bool {
    internal::elkodon_fd_isset(fd, set) != 0
}

pub unsafe fn FD_SET(fd: int, set: *mut fd_set) {
    internal::elkodon_fd_set(fd, set)
}

pub unsafe fn FD_ZERO(set: *mut fd_set) {
    internal::elkodon_fd_zero(set)
}

mod internal {
    use super::*;

    #[cfg_attr(target_os = "linux", link(name = "c"))]
    extern "C" {
        pub(super) fn elkodon_cmsg_space(len: size_t) -> size_t;
        pub(super) fn elkodon_cmsg_firsthdr(hdr: *const msghdr) -> *mut cmsghdr;
        pub(super) fn elkodon_cmsg_nxthdr(hdr: *mut msghdr, sub: *mut cmsghdr) -> *mut cmsghdr;
        pub(super) fn elkodon_cmsg_len(len: size_t) -> size_t;
        pub(super) fn elkodon_cmsg_data(cmsg: *mut cmsghdr) -> *mut uchar;
        pub(super) fn elkodon_fd_clr(fd: int, set: *mut fd_set);
        pub(super) fn elkodon_fd_isset(fd: int, set: *const fd_set) -> int;
        pub(super) fn elkodon_fd_set(fd: int, set: *mut fd_set);
        pub(super) fn elkodon_fd_zero(set: *mut fd_set);
    }
}
