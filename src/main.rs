use bindings::Windows::Win32::{Foundation::{HANDLE,}, Security::{SECURITY_ATTRIBUTES,}, Storage::FileSystem::{CreateFileW, FILE_ACCESS_FLAGS, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, READ_USN_JOURNAL_DATA_V0, USN_JOURNAL_DATA_V0}, System::SystemServices::{DeviceIoControl, FSCTL_QUERY_USN_JOURNAL}};


fn main() {
    println!("Hello, world!");
    let handle = CreateFileW(
        "\\.\E:", 
    FILE_GENERIC_READ , 
    FILE_SHARE_READ|FILE_SHARE_WRITE, 
    lpsecurityattributes, dwcreationdisposition, dwflagsandattributes, htemplatefile)
}
