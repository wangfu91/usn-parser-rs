use std::{ffi::OsString, os::windows::ffi::OsStringExt};

use windows::Win32::{
    Storage::FileSystem::{
        FILE_ATTRIBUTE_DIRECTORY, FILE_ATTRIBUTE_HIDDEN, FILE_FLAGS_AND_ATTRIBUTES,
    },
    System::Ioctl::USN_RECORD_V2,
};

use chrono::{DateTime, FixedOffset};

use crate::utils;

#[derive(Debug)]
pub struct UsnEntry {
    record_length: u32,
    pub usn: i64,
    pub timestamp: DateTime<FixedOffset>,
    pub fid: u64,
    pub parent_fid: u64,
    pub reason: u32,
    pub source_info: u32,
    pub file_name: OsString,
    pub file_attributes: FILE_FLAGS_AND_ATTRIBUTES,
}

impl UsnEntry {
    pub fn new(record: &USN_RECORD_V2) -> Self {
        let file_name_len = record.FileNameLength as usize / std::mem::size_of::<u16>();

        // https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-usn_record_v2
        // When working with FileName, do not count on the file name that contains a trailing '\0' delimiter,
        // but instead determine the length of the file name by using FileNameLength.
        // Do not perform any compile-time pointer arithmetic using FileName.
        // Instead, make necessary calculations at run time by using the value of the FileNameOffset member.
        // Doing so helps make your code compatible with any future versions of USN_RECORD_V2.
        let file_name_data =
            unsafe { std::slice::from_raw_parts(record.FileName.as_ptr(), file_name_len) };
        let file_name = OsString::from_wide(file_name_data);

        UsnEntry {
            record_length: record.RecordLength,
            usn: record.Usn,
            timestamp: utils::filetime_to_datetime(record.TimeStamp),
            fid: record.FileReferenceNumber,
            parent_fid: record.ParentFileReferenceNumber,
            reason: record.Reason,
            source_info: record.SourceInfo,
            file_name,
            file_attributes: FILE_FLAGS_AND_ATTRIBUTES(record.FileAttributes),
        }
    }

    pub fn is_dir(&self) -> bool {
        self.file_attributes.contains(FILE_ATTRIBUTE_DIRECTORY)
    }

    pub fn is_hidden(&self) -> bool {
        self.file_attributes.contains(FILE_ATTRIBUTE_HIDDEN)
    }
}
