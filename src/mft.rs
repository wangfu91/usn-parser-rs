use std::{ffi::OsString, num::NonZeroUsize, os::windows::ffi::OsStringExt, path::PathBuf};

use lru::LruCache;
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

use crate::{
    usn_entry::UsnEntry,
    utils::{self, file_id_to_path},
};

type Usn = i64;

pub struct MFT {
    pub volume_handle: HANDLE,
    high_usn: Usn,
    pub buffer: [u8; 64 * 1024],
    pub usn_record: Option<USN_RECORD_V2>,
    pub bytes_read: u32,
    pub offset: u32,
    next_start_fid: u64,
}

impl MFT {
    pub fn new(volume_handle: HANDLE, high_usn: Usn) -> Self {
        MFT {
            volume_handle,
            high_usn,
            buffer: [0u8; 64 * 1024],
            usn_record: None,
            bytes_read: 0,
            offset: 0,
            next_start_fid: 0,
        }
    }

    fn get_data(&mut self) -> anyhow::Result<bool> {
        println!("get_data, next_start_fid = {}", self.next_start_fid);
        // To enumerate files on a volume, use the FSCTL_ENUM_USN_DATA operation one or more times.
        // On the first call, set the starting point, the StartFileReferenceNumber member of the MFT_ENUM_DATA structure, to (DWORDLONG)0.
        let mft_enum_data = Ioctl::MFT_ENUM_DATA_V0 {
            StartFileReferenceNumber: self.next_start_fid,
            LowUsn: 0,
            HighUsn: self.high_usn,
        };

        if let Err(err) = unsafe {
            DeviceIoControl(
                self.volume_handle,
                Ioctl::FSCTL_ENUM_USN_DATA,
                Some(&mft_enum_data as *const _ as _),
                size_of::<Ioctl::MFT_ENUM_DATA_V0>() as u32,
                Some(self.buffer.as_mut_ptr() as _),
                self.buffer.len() as u32,
                Some(&mut self.bytes_read),
                None,
            )
        } {
            if err.code() == ERROR_HANDLE_EOF.into() {
                println!("Done reading MFT data");
                return Ok(false);
            }

            println!("Error reading MFT data: {}", err);
            return Err(err.into());
        }

        println!("Read {} bytes from MFT", self.bytes_read);

        Ok(true)
    }

    fn find_next_entry(&mut self) -> anyhow::Result<()> {
        println!("find_next_entry, offset = {}", self.offset);

        if self.usn_record.is_some() && self.offset < self.bytes_read {
            let record = unsafe {
                &*(self.buffer.as_ptr().offset(self.offset as isize) as *const USN_RECORD_V2)
                /*
                std::ptr::read(
                    self.buffer.as_ptr().offset(self.offset as isize) as *const USN_RECORD_V2
                )
                 */
            };
            self.usn_record = Some(*record);
            self.offset += record.RecordLength;
            return Ok(());
        }

        // We need to read more data
        if self.get_data()? {
            // Each call to FSCTL_ENUM_USN_DATA retrieves the starting point for the subsequent call as the first entry in the output buffer.
            self.next_start_fid = unsafe { std::ptr::read(self.buffer.as_ptr() as *const u64) };
            println!("next_start_fid = {}", self.next_start_fid);
            self.offset = std::mem::size_of::<u64>() as u32;
            println!("offset = {}", self.offset);
            if self.offset < self.bytes_read {
                let record = unsafe {
                    &*(self.buffer.as_ptr().offset(self.offset as isize) as *const USN_RECORD_V2)
                    /*
                    std::ptr::read(
                        self.buffer.as_ptr().offset(self.offset as isize) as *const USN_RECORD_V2
                    )
                     */
                };
                self.usn_record = Some(*record);
                self.offset += record.RecordLength;
                return Ok(());
            }
        }

        // EOF, no more data to read
        self.usn_record = None;
        Ok(())
    }
}

impl Iterator for MFT {
    type Item = UsnEntry;

    fn next(&mut self) -> Option<Self::Item> {
        println!("next, offset = {}", self.offset);

        if let Err(err) = self.find_next_entry() {
            println!("Error finding next entry: {}", err);
            return None;
        }

        if self.usn_record.is_none() {
            None
        } else {
            let record = self.usn_record.unwrap();

            let file_name_len = record.FileNameLength as usize / std::mem::size_of::<u16>();

            // https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ns-winioctl-usn_record_v2
            // When working with FileName, do not count on the file name that contains a trailing '\0' delimiter,
            // but instead determine the length of the file name by using FileNameLength.
            // Do not perform any compile-time pointer arithmetic using FileName.
            // Instead, make necessary calculations at run time by using the value of the FileNameOffset member.
            // Doing so helps make your code compatible with any future versions of USN_RECORD_V2.
            let file_name_u16 = unsafe {
                std::slice::from_raw_parts(
                    self.buffer
                        .as_ptr()
                        .offset((self.offset + record.FileNameOffset as u32) as isize)
                        as *const u16,
                    file_name_len,
                )
            };
            let file_name = OsString::from_wide(file_name_u16);
            println!("====== file_name = {}", file_name.to_string_lossy());
            let enrty = UsnEntry::new(&record);
            Some(enrty)
        }
    }
}

pub fn read_mft(volume_handle: HANDLE, journal_data: &USN_JOURNAL_DATA_V0) -> anyhow::Result<()> {
    let mut mft_enum_data = Ioctl::MFT_ENUM_DATA_V0 {
        StartFileReferenceNumber: 0,
        LowUsn: 0,
        HighUsn: journal_data.NextUsn,
    };
    let mut buffer = [0u8; 256 * 1024];
    let mut bytes_read: u32 = 0;

    let mut cache: LruCache<u64, PathBuf> = LruCache::new(NonZeroUsize::new(4 * 1024).unwrap());

    loop {
        let enum_result = unsafe {
            DeviceIoControl(
                volume_handle,
                Ioctl::FSCTL_ENUM_USN_DATA,
                Some(&mft_enum_data as *const _ as _),
                size_of::<Ioctl::MFT_ENUM_DATA_V0>() as u32,
                Some(buffer.as_mut_ptr() as _),
                buffer.len() as u32,
                Some(&mut bytes_read),
                None,
            )
        };

        match enum_result {
            Ok(_) => {}
            Err(err) => {
                if err.code() == ERROR_HANDLE_EOF.into() {
                    println!("Done reading MFT data");
                    return Ok(());
                }

                return Err(err.into());
            }
        }

        let next_start_file_id = unsafe { std::ptr::read(buffer.as_ptr() as *const Usn) };
        mft_enum_data.StartFileReferenceNumber = next_start_file_id as u64;

        let mut offset = std::mem::size_of::<Usn>() as u32;
        while offset < bytes_read {
            let record = unsafe {
                std::ptr::read(buffer.as_ptr().offset(offset as isize) as *const USN_RECORD_V2)
            };

            if record.RecordLength == 0 || record.MajorVersion != 2 {
                println!("Unsupported USN major version: {}", record.MajorVersion);
                break;
            }

            if let Some(record_path) = cache.get(&record.FileReferenceNumber) {
                // println!(
                //     "MFT record cached, ({},{})",
                //     record.FileReferenceNumber,
                //     record_path.display()
                // );
                offset += record.RecordLength;
                continue;
            }

            let file_name_len = record.FileNameLength as usize / std::mem::size_of::<u16>();

            // Do not perform any compile-time pointer arithmetic using FileName.
            // Instead, make necessary calculations at run time by using the value of the FileNameOffset member.
            // Doing so helps make your code compatible with any future versions of USN_RECORD_V2.
            let file_name_u16 = unsafe {
                std::slice::from_raw_parts(
                    buffer
                        .as_ptr()
                        .offset((offset + record.FileNameOffset as u32) as isize)
                        as *const u16,
                    file_name_len,
                )
            };
            let file_name = OsString::from_wide(file_name_u16);

            if let Some(parent_path) = cache.get(&record.ParentFileReferenceNumber) {
                let record_path = parent_path.join(file_name);
                //println!("MFT record 1, path = {}", record_path.display());
            } else if let Ok(parent_path) =
                utils::file_id_to_path(volume_handle, record.ParentFileReferenceNumber)
            {
                // println!(
                //     "+ ({},{})",
                //     record.ParentFileReferenceNumber,
                //     parent_path.display()
                // );
                cache.put(record.ParentFileReferenceNumber, parent_path.clone());
                let record_path = parent_path.join(file_name);

                if record.FileAttributes & FileSystem::FILE_ATTRIBUTE_DIRECTORY.0 != 0 {
                    // println!(
                    //     "++ ({},{})",
                    //     record.FileReferenceNumber,
                    //     record_path.display()
                    // );
                    cache.put(record.FileReferenceNumber, record_path.clone());
                }

                println!(
                    "MFT record 2, ({},{})",
                    record.FileReferenceNumber,
                    record_path.display()
                );
            } else {
                //Skip files we don't have access to
            }

            offset += record.RecordLength;
        }

        if offset != bytes_read {
            panic!("offset vs bytesRead: {} vs {}", offset, bytes_read);
        }
    }
}
