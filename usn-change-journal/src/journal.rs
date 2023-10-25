use lru::LruCache;
use std::num::NonZeroUsize;

use std::{
    default::Default,
    ffi::{c_void, OsString},
    mem::{self, size_of},
    os::windows::prelude::OsStringExt,
    path::PathBuf,
    slice,
};

use anyhow::Context;
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{self, ERROR_HANDLE_EOF, HANDLE},
        Storage::FileSystem::{
            self, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_ID_DESCRIPTOR,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            Ioctl::{
                self, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
                USN_JOURNAL_DATA_V0, USN_RECORD_V2,
            },
            IO::DeviceIoControl,
        },
    },
};

use crate::handle::SafeFileHandle;

type Usn = i64;

pub struct UsnJournal {}

impl UsnJournal {
    pub fn get_volume_handle(&self, volume_root: &str) -> anyhow::Result<SafeFileHandle> {
        let volume_handle = unsafe {
            CreateFileW(
                &HSTRING::from(volume_root),
                FILE_GENERIC_READ.0,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                None,
                OPEN_EXISTING,
                FILE_FLAGS_AND_ATTRIBUTES::default(),
                HANDLE::default(),
            )?
        };

        Ok(SafeFileHandle(volume_handle))
    }

    pub fn query_usn_state(
        &self,
        volume_handle: &SafeFileHandle,
    ) -> anyhow::Result<USN_JOURNAL_DATA_V0> {
        let journal_data = USN_JOURNAL_DATA_V0::default();
        let query_journal_bytes_return = 0u32;

        unsafe {
            DeviceIoControl(
                volume_handle.0,
                FSCTL_QUERY_USN_JOURNAL,
                None,
                0,
                Some(&journal_data as *const _ as *mut _),
                mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
                Some(&query_journal_bytes_return as *const _ as *mut _),
                None,
            )
        }?;

        Ok(journal_data)
    }

    pub fn read_mft(
        &self,
        volume_handle: &SafeFileHandle,
        journal_data: &USN_JOURNAL_DATA_V0,
    ) -> anyhow::Result<()> {
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
                    volume_handle.0,
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
                    slice::from_raw_parts(
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
                    Self::file_id_to_path(volume_handle, record.ParentFileReferenceNumber)
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

                    // println!(
                    //     "MFT record 2, ({},{})",
                    //     record.FileReferenceNumber,
                    //     record_path.display()
                    // );
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

    pub fn monitor_usn_journal(
        &self,
        volume_handle: &SafeFileHandle,
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
                    volume_handle.0,
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
                    Self::file_id_to_path(volume_handle, record.ParentFileReferenceNumber)
                {
                    let record_path = parent_path.join(file_name);

                    println!("record path = {}", record_path.display());
                }

                offset += record.RecordLength;
            }
        }
    }

    fn file_id_to_path(volume_handle: &SafeFileHandle, file_id: u64) -> anyhow::Result<PathBuf> {
        let file_id_desc = FILE_ID_DESCRIPTOR {
            Type: FileSystem::FileIdType,
            dwSize: size_of::<FileSystem::FILE_ID_DESCRIPTOR>() as u32,
            Anonymous: FileSystem::FILE_ID_DESCRIPTOR_0 {
                FileId: file_id.try_into()?,
            },
        };

        let file_handle = unsafe {
            FileSystem::OpenFileById(
                volume_handle.0,
                &file_id_desc,
                FileSystem::FILE_GENERIC_READ.0,
                FileSystem::FILE_SHARE_READ
                    | FileSystem::FILE_SHARE_WRITE
                    | FileSystem::FILE_SHARE_DELETE,
                None,
                FILE_FLAGS_AND_ATTRIBUTES::default(),
            )
            .with_context(|| format!("OpenFileById failed, file_id={}", file_id))?
        };

        let init_len = size_of::<u32>() + (Foundation::MAX_PATH as usize) * size_of::<u16>();
        let mut info_buffer = vec![0u8; init_len];

        loop {
            if let Err(err) = unsafe {
                FileSystem::GetFileInformationByHandleEx(
                    file_handle,
                    FileSystem::FileNameInfo,
                    &mut *info_buffer as *mut _ as *mut c_void,
                    info_buffer.len() as u32,
                )
            } {
                if err.code() == Foundation::ERROR_MORE_DATA.into() {
                    // Long paths, needs to extend buffer size to hold it.
                    let name_info = unsafe {
                        std::ptr::read(info_buffer.as_ptr() as *const FileSystem::FILE_NAME_INFO)
                    };

                    let needed_len = name_info.FileNameLength + size_of::<u32>() as u32;
                    // expand info_buffer capacity to needed_len to hold the long path
                    info_buffer.resize(needed_len as usize, 0);
                    // try again
                    continue;
                }

                return Err(err.into());
            }

            break;
        }

        unsafe { Foundation::CloseHandle(file_handle) }?;

        let (_, body, _) = unsafe { info_buffer.align_to::<FileSystem::FILE_NAME_INFO>() };
        let info = &body[0];
        let name_len = info.FileNameLength as usize / size_of::<u16>();
        let name_u16 = unsafe { slice::from_raw_parts(info.FileName.as_ptr(), name_len) };
        let path = PathBuf::from(OsString::from_wide(name_u16));
        Ok(path)
    }
}
