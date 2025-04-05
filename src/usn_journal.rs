use windows::Win32::{
    Foundation::HANDLE,
    System::{
        IO::DeviceIoControl,
        Ioctl::{FSCTL_QUERY_USN_JOURNAL, USN_JOURNAL_DATA_V0},
    },
};

pub fn query_usn_info(volume_handle: HANDLE) -> anyhow::Result<USN_JOURNAL_DATA_V0> {
    let journal_data = USN_JOURNAL_DATA_V0::default();
    let bytes_return = 0u32;

    unsafe {
        DeviceIoControl(
            volume_handle,
            FSCTL_QUERY_USN_JOURNAL,
            None,
            0,
            Some(&journal_data as *const _ as *mut _),
            std::mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
            Some(&bytes_return as *const _ as *mut _),
            None,
        )
    }?;

    Ok(journal_data)
}
