fn main() {
    windows::build! {
        Windows::Win32::Storage::FileSystem::{
            CreateFileW,
            USN_JOURNAL_DATA_V0,
            USN_RECORD_V2,
            READ_USN_JOURNAL_DATA_V0,
            FILE_ACCESS_FLAGS,
            FILE_SHARE_MODE,
            FILE_CREATION_DISPOSITION,
            FILE_FLAGS_AND_ATTRIBUTES,            
        },

        Windows::Win32::System::SystemServices:: {
            DeviceIoControl,
            FSCTL_QUERY_USN_JOURNAL,
            FSCTL_READ_USN_JOURNAL,
            
        },

        Windows::Win32::Security::{
            SECURITY_ATTRIBUTES,
        },

        Windows::Win32::Foundation::{
            HANDLE,
        },

        Windows::Win32::System::Diagnostics::Debug::{
            GetLastError,
            WIN32_ERROR,
        },
    };
}
