use windows_sys::Win32::{
    Foundation::{
        ERROR_ACCESS_DENIED, ERROR_ALREADY_EXISTS, ERROR_ARENA_TRASHED, ERROR_BAD_COMMAND,
        ERROR_BAD_LENGTH, ERROR_CURRENT_DIRECTORY, ERROR_DEV_NOT_EXIST, ERROR_FILE_EXISTS,
        ERROR_FILE_NOT_FOUND, ERROR_FILE_TOO_LARGE, ERROR_HANDLE_DISK_FULL, ERROR_INVALID_ACCESS,
        ERROR_INVALID_BLOCK, ERROR_INVALID_DATA, ERROR_INVALID_HANDLE, ERROR_NOT_ENOUGH_MEMORY,
        ERROR_NOT_READY, ERROR_OUTOFMEMORY, ERROR_PATH_NOT_FOUND, ERROR_READ_FAULT,
        ERROR_SECTOR_NOT_FOUND, ERROR_SHARING_BUFFER_EXCEEDED, ERROR_SUCCESS,
        ERROR_TOO_MANY_OPEN_FILES, ERROR_WRITE_FAULT, ERROR_WRITE_PROTECT, WIN32_ERROR,
    },
    Networking::WinSock::{
        WSAEACCES, WSAEADDRINUSE, WSAEADDRNOTAVAIL, WSAEBADF, WSAECONNABORTED, WSAECONNREFUSED,
        WSAECONNRESET, WSAEHOSTUNREACH, WSAEINTR, WSAEINVAL, WSAEISCONN, WSAEMFILE, WSAEMSGSIZE,
        WSAENETDOWN, WSAENETRESET, WSAENETUNREACH, WSAENOBUFS, WSAENOPROTOOPT, WSAEPROTONOSUPPORT,
        WSAEPROTOTYPE, WSAETIMEDOUT, WSAEWOULDBLOCK, WSA_ERROR, WSA_INVALID_HANDLE,
        WSA_INVALID_PARAMETER, WSA_IO_INCOMPLETE, WSA_IO_PENDING, WSA_NOT_ENOUGH_MEMORY,
    },
};

use crate::posix::Errno;

pub unsafe fn system_error_code_to_errno(value: WIN32_ERROR) {
    match value {
        ERROR_SUCCESS => Errno::set(Errno::ESUCCES),
        ERROR_FILE_NOT_FOUND | ERROR_PATH_NOT_FOUND => Errno::set(Errno::ENOENT),
        ERROR_TOO_MANY_OPEN_FILES | ERROR_SHARING_BUFFER_EXCEEDED => Errno::set(Errno::EMFILE),
        ERROR_ACCESS_DENIED => Errno::set(Errno::EACCES),
        ERROR_INVALID_HANDLE => Errno::set(Errno::EBADF),
        ERROR_ARENA_TRASHED
        | ERROR_INVALID_BLOCK
        | ERROR_WRITE_FAULT
        | ERROR_READ_FAULT
        | ERROR_SECTOR_NOT_FOUND => Errno::set(Errno::EIO),
        ERROR_NOT_ENOUGH_MEMORY | ERROR_FILE_TOO_LARGE => Errno::set(Errno::ENOSPC),
        ERROR_OUTOFMEMORY => Errno::set(Errno::ENOMEM),
        ERROR_INVALID_ACCESS | ERROR_INVALID_DATA => Errno::set(Errno::EINVAL),
        ERROR_CURRENT_DIRECTORY | ERROR_NOT_READY => Errno::set(Errno::EBUSY),
        ERROR_WRITE_PROTECT => Errno::set(Errno::EROFS),
        ERROR_BAD_COMMAND | ERROR_BAD_LENGTH => Errno::set(Errno::EINVAL),
        ERROR_HANDLE_DISK_FULL => Errno::set(Errno::ENOBUFS),
        ERROR_DEV_NOT_EXIST => Errno::set(Errno::ENODEV),
        ERROR_ALREADY_EXISTS | ERROR_FILE_EXISTS => Errno::set(Errno::EEXIST),

        _ => Errno::set(Errno::EINVAL),
    }
}

pub unsafe fn wsa_to_errno(value: WSA_ERROR) {
    match value {
        0 => Errno::set(Errno::ESUCCES),
        WSA_INVALID_HANDLE => Errno::set(Errno::EBADF),
        WSA_NOT_ENOUGH_MEMORY => Errno::set(Errno::ENOMEM),
        WSA_INVALID_PARAMETER => Errno::set(Errno::EINVAL),
        WSA_IO_INCOMPLETE | WSA_IO_PENDING => Errno::set(Errno::EIO),
        WSAEWOULDBLOCK => Errno::set(Errno::EAGAIN),
        WSAETIMEDOUT => Errno::set(Errno::ETIMEDOUT),
        WSAEINTR => Errno::set(Errno::EINTR),
        WSAEBADF => Errno::set(Errno::EBADF),
        WSAEACCES => Errno::set(Errno::EACCES),
        WSAEINVAL => Errno::set(Errno::EINVAL),
        WSAEMFILE => Errno::set(Errno::EMFILE),
        WSAEMSGSIZE => Errno::set(Errno::EMSGSIZE),
        WSAEPROTOTYPE => Errno::set(Errno::EPROTOTYPE),
        WSAENOPROTOOPT => Errno::set(Errno::ENOPROTOOPT),
        WSAEPROTONOSUPPORT => Errno::set(Errno::EPROTONOSUPPORT),
        WSAEADDRINUSE => Errno::set(Errno::EADDRINUSE),
        WSAEADDRNOTAVAIL => Errno::set(Errno::EADDRNOTAVAIL),
        WSAENETDOWN => Errno::set(Errno::ENETDOWN),
        WSAENETUNREACH => Errno::set(Errno::ENETUNREACH),
        WSAENETRESET => Errno::set(Errno::ENETRESET),
        WSAECONNABORTED => Errno::set(Errno::ECONNABORTED),
        WSAECONNRESET => Errno::set(Errno::ECONNRESET),
        WSAECONNREFUSED => Errno::set(Errno::ECONNREFUSED),
        WSAENOBUFS => Errno::set(Errno::ENOBUFS),
        WSAEISCONN => Errno::set(Errno::EISCONN),
        WSAEHOSTUNREACH => Errno::set(Errno::EHOSTUNREACH),
        _ => Errno::set(Errno::EINVAL),
    }
}

#[macro_export(local_inner_macros)]
macro_rules! win32call {
   {$call:expr, ignore $($error:ident),*} => {
        {
            windows_sys::Win32::Foundation::SetLastError(0);
            let ret_val = $call;
            let last_error = windows_sys::Win32::Foundation::GetLastError();
            if last_error != 0 {
                $crate::windows::win32_call::system_error_code_to_errno(last_error);
                match last_error {
                    $($error => ()),*,
                    _ => {
                        let mut buffer = [0u8; 1024];
                        windows_sys::Win32::System::Diagnostics::Debug::FormatMessageA(
                            windows_sys::Win32::System::Diagnostics::Debug::FORMAT_MESSAGE_FROM_SYSTEM |
                            windows_sys::Win32::System::Diagnostics::Debug::FORMAT_MESSAGE_IGNORE_INSERTS,
                            core::ptr::null::<void>(),
                            last_error,
                            0,
                            buffer.as_mut_ptr(),
                            buffer.len() as u32,
                            core::ptr::null::<*const i8>()
                        );
                        std::println!(
                            "< Win32 API error > {}:{} {} \n [ {} ] {}",
                            std::file!(), std::line!(), std::stringify!($call), last_error,
                            std::str::from_utf8(&buffer).unwrap()
                        );
                    },
                }

           }

            ret_val
        }
    };
    {$call:expr} => {
        {
            use windows_sys::Win32::Foundation::WIN32_ERROR;
            const NO_ERROR: WIN32_ERROR = 0;
            win32call!($call, ignore NO_ERROR)
        }
    };
    {winsock $call:expr, ignore $($error:ident),*} => {
        {
            windows_sys::Win32::Networking::WinSock::WSASetLastError(0);
            let ret_val = $call;
            let last_error = windows_sys::Win32::Networking::WinSock::WSAGetLastError();
            if last_error != 0 {
                $crate::windows::win32_call::wsa_to_errno(last_error);
                match last_error {
                    $($error => ()),*,
                    _ => {
                        let mut buffer = [0u8; 1024];
                        windows_sys::Win32::System::Diagnostics::Debug::FormatMessageA(
                            windows_sys::Win32::System::Diagnostics::Debug::FORMAT_MESSAGE_FROM_SYSTEM |
                            windows_sys::Win32::System::Diagnostics::Debug::FORMAT_MESSAGE_IGNORE_INSERTS,
                            core::ptr::null::<void>(),
                            last_error as _,
                            0,
                            buffer.as_mut_ptr(),
                            buffer.len() as u32,
                            core::ptr::null::<*const i8>(),
                        );
                        std::println!(
                            "< Win32 WinSock2 API error > {}:{} {} \n [ {} ] {}",
                            std::file!(), std::line!(), std::stringify!($call), last_error,
                            std::str::from_utf8(&buffer).unwrap()
                        );
                    },
                }

           }

            ret_val
        }
    };
    {winsock $call:expr} => {
        {
            use windows_sys::Win32::Networking::WinSock::WSA_ERROR;
            const NO_ERROR: WSA_ERROR = 0;
            win32call!(winsock $call, ignore NO_ERROR)
        }
    };
}
