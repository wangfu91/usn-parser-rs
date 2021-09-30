use std::{ffi::c_void, mem, ptr};

use bindings::Windows::Win32::{
    Foundation::HANDLE,
    Security::SECURITY_ATTRIBUTES,
    Storage::FileSystem::{
        CreateFileW, FILE_ACCESS_FLAGS, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ, FILE_SHARE_MODE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING, READ_USN_JOURNAL_DATA_V0, USN_JOURNAL_DATA_V0,
    },
    System::{
        Diagnostics::Debug::GetLastError,
        SystemServices::{DeviceIoControl, FSCTL_QUERY_USN_JOURNAL},
    },
};

fn main() {
    // https://microsoft.github.io/windows-docs-rs/doc/bindings/Windows/Win32/System/Diagnostics/Debug/struct.WIN32_ERROR.html
    unsafe {
        let hVol = CreateFileW(
            "\\\\.\\D:",
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &SECURITY_ATTRIBUTES::default(),
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE(0),
        );

        println!("hVol = {:?}", hVol);
        let error = GetLastError();
        println!("last error: {:?}", error);

        let journalData = USN_JOURNAL_DATA_V0::default();
        let bytesReturn = 0u64;

        unsafe {
            let result = DeviceIoControl(
                hVol,
                FSCTL_QUERY_USN_JOURNAL,
                ptr::null_mut(),
                0,
                &journalData as *const _ as *mut _,
                mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
                &bytesReturn as *const _ as *mut _,
                ptr::null_mut(),
            );

            println!("Query change journal result: {:?}", result);

            let error = GetLastError();
            println!("last error: {:?}", error);

            println!("Journal data: {:?}", journalData);
            println!("bytesReturn: {:?}", bytesReturn);
        };
    }
}
