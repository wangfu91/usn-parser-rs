use lru::LruCache;
use std::num::NonZeroUsize;

use std::{
    default::Default,
    ffi::{OsString, c_void},
    mem::{self, size_of},
    os::windows::prelude::OsStringExt,
    path::PathBuf,
    slice,
};

use anyhow::Context;
use windows::{
    Win32::{
        Foundation::{self, ERROR_HANDLE_EOF, HANDLE},
        Storage::FileSystem::{
            self, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_ID_DESCRIPTOR,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            IO::DeviceIoControl,
            Ioctl::{
                self, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
                USN_JOURNAL_DATA_V0, USN_RECORD_V2,
            },
        },
    },
    core::HSTRING,
};

use crate::{usn_info, utils};

type Usn = i64;

pub struct UsnJournal {
    pub volume: String,
    pub volume_handle: HANDLE,
    pub journal_data: USN_JOURNAL_DATA_V0,
}

impl UsnJournal {
    pub fn new(volume: &str) -> anyhow::Result<Self> {
        let volume_handle = utils::get_volume_handle(volume)?;
        let journal_data = usn_info::query_usn_info(volume_handle)?;

        Ok(Self {
            volume: volume.to_string(),
            volume_handle,
            journal_data,
        })
    }
}

pub fn monitor_usn_journal(
    volume_handle: HANDLE,
    journal_data: &USN_JOURNAL_DATA_V0,
) -> anyhow::Result<()> {
    let mut read_data = READ_USN_JOURNAL_DATA_V0 {
        StartUsn: journal_data.NextUsn,
        ReasonMask: 0xFFFFFFFF,
        ReturnOnlyOnClose: 0,
        Timeout: 0,
        BytesToWaitFor: 1,
        UsnJournalID: journal_data.UsnJournalID,
    };

    let mut buffer = [0u8; 4096];
    let mut read_data_bytes_return = 0u32;

    loop {
        unsafe {
            DeviceIoControl(
                volume_handle,
                FSCTL_READ_USN_JOURNAL,
                Some(&read_data as *const _ as *mut _),
                size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(buffer.as_mut_ptr() as *mut c_void),
                4096,
                Some(&mut read_data_bytes_return),
                None,
            )?
        };

        // https://learn.microsoft.com/en-us/windows/win32/fileio/walking-a-buffer-of-change-journal-records
        // The USN returned as the first item in the output buffer is the USN of the next record number to be retrieved.
        // Use this value to continue reading records from the end boundary forward.
        let next_usn = unsafe { std::ptr::read(buffer.as_ptr() as *const Usn) };
        if next_usn == 0 || next_usn < read_data.StartUsn {
            return Ok(());
        } else {
            read_data.StartUsn = next_usn;
        }

        let mut offset = size_of::<Usn>() as u32;

        while offset < read_data_bytes_return {
            let record = unsafe {
                std::ptr::read(buffer.as_ptr().offset(offset as isize) as *const USN_RECORD_V2)
            };

            if record.RecordLength == 0 || record.MajorVersion != 2 {
                println!("Unsupported USN major version: {}", record.MajorVersion);
                break;
            }

            let file_name_len = record.FileNameLength as usize / std::mem::size_of::<u16>();

            // Do not perform any compile-time pointer arithmetic using FileName.
            // Instead, make necessary calculations at run time by using the value of the FileNameOffset member.
            // Doing so helps make your code compatible with any future versions of USN_RECORD_V2.
            let file_name_u16 = unsafe {
                slice::from_raw_parts(
                    buffer
                        .as_ptr()
                        .offset((offset + record.FileNameOffset as u32) as isize)
                        as *const u16,
                    file_name_len,
                )
            };
            let file_name = OsString::from_wide(file_name_u16);

            if let Ok(parent_path) =
                file_id_to_path(volume_handle, record.ParentFileReferenceNumber)
            {
                let record_path = parent_path.join(file_name);

                println!("record path = {}", record_path.display());
            }

            offset += record.RecordLength;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::{file_id_to_path, get_volume_handle};

    #[test]
    fn get_long_path_test() -> anyhow::Result<()> {
        let volume = "C:";
        let volume_handle = get_volume_handle(volume)?;

        let file_id = 5066549581896872u64;

        let path = file_id_to_path(volume_handle, file_id)?;

        println!("path = {:#?}", path);

        Ok(())
    }

    #[test]
    fn iter_test() {}
}
