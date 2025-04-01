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

use crate::utils::file_id_to_path;

type Usn = i64;

pub struct MFT {
    pub volume_handle: HANDLE,
    pub journal_data: USN_JOURNAL_DATA_V0,
    // pub mft_enum_data: Ioctl::MFT_ENUM_DATA_V0,
    // pub buffer: [u8; 256 * 1024],
    // pub bytes_read: u32,
    // pub cache: LruCache<u64, PathBuf>,
    // pub offset: u32,
}

impl MFT {
    pub fn new(volume_handle: HANDLE, journal_data: USN_JOURNAL_DATA_V0) -> Self {
        MFT {
            volume_handle,
            journal_data,
        }
    }

    fn get_data(&self) {
        let mut mft_enum_data = Ioctl::MFT_ENUM_DATA_V0 {
            StartFileReferenceNumber: 0,
            LowUsn: 0,
            HighUsn: self.journal_data.NextUsn,
        };
        let mut buffer = [0u8; 256 * 1024];
        let mut bytes_read: u32 = 0;

        let mut cache: LruCache<u64, PathBuf> = LruCache::new(NonZeroUsize::new(4 * 1024).unwrap());

        let enum_result = unsafe {
            DeviceIoControl(
                self.volume_handle,
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
                file_id_to_path(volume_handle, record.ParentFileReferenceNumber)
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
