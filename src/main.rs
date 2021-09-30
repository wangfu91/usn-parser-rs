use bindings::Windows::Win32::{Foundation::{HANDLE,}, Security::{SECURITY_ATTRIBUTES,}, Storage::FileSystem::{CreateFileW, FILE_ACCESS_FLAGS, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES, FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ, FILE_SHARE_MODE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, READ_USN_JOURNAL_DATA_V0, USN_JOURNAL_DATA_V0}, System::SystemServices::{DeviceIoControl, FSCTL_QUERY_USN_JOURNAL}};
use std::ptr;

fn main() {
    println!("Hello, world!");
    
    unsafe {
    let handle = CreateFileW(
        "\\\\.\\D:", 
    FILE_GENERIC_READ , 
    FILE_SHARE_READ|FILE_SHARE_WRITE, 
    ptr::null_mut(), OPEN_EXISTING, FILE_FLAG_BACKUP_SEMANTICS, HANDLE(0));

    println!("handle = {:?}", handle);
    }   

}
