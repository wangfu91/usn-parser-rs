use std::{
    default::Default,
    ffi::{c_void, OsString},
    mem::{self, size_of, transmute},
    os::windows::prelude::OsStringExt,
    path::PathBuf,
    ptr, slice,
};

use windows::{
    core::{Error, HSTRING},
    Win32::{
        Foundation::{self, HANDLE},
        Storage::FileSystem::{
            self, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_ID_128,
            FILE_ID_DESCRIPTOR, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
        System::{
            Ioctl::{
                FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
                USN_JOURNAL_DATA_V0, USN_RECORD_UNION, USN_RECORD_V2,
            },
            IO::DeviceIoControl,
        },
    },
};

use byteorder::{ByteOrder, LittleEndian};

fn main() -> anyhow::Result<(), Error> {
    let volume = "D:";
    let volumne_root = format!(r"\\.\{}", volume);
    println!("volume_root={}", volumne_root);

    // https://microsoft.github.io/windows-docs-rs/doc/bindings/Windows/Win32/System/Diagnostics/Debug/struct.WIN32_ERROR.html

    let volume_handle = unsafe {
        CreateFileW(
            &HSTRING::from(volumne_root),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES::default(),
            HANDLE::default(),
        )?
    };

    println!("volume handle = {:?}", volume_handle);

    let journal_data = USN_JOURNAL_DATA_V0::default();
    let query_journal_bytes_return = 0u32;
    let mut next_usn = 0i64;
    let mut journal_id = 0u64;

    unsafe {
        let succeess = DeviceIoControl(
            volume_handle,
            FSCTL_QUERY_USN_JOURNAL,
            None,
            0,
            Some(&journal_data as *const _ as *mut _),
            mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
            Some(&query_journal_bytes_return as *const _ as *mut _),
            None,
        );

        println!("Query change journal result: {:?}", succeess);

        if !succeess.as_bool() {
            let error = Error::from_win32();
            println!("last error: {:?}", error);
            return Err(error);
        }

        println!("Journal data: {:#?}", journal_data);
        println!("bytesReturn: {:?}", query_journal_bytes_return);

        next_usn = journal_data.NextUsn;
        journal_id = journal_data.UsnJournalID;
    };

    // Read USN journal

    loop {
        let read_data = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: next_usn,
            ReasonMask: 0xFFFFFFFF,
            ReturnOnlyOnClose: 0,
            Timeout: 0,
            BytesToWaitFor: 1,
            UsnJournalID: journal_id,
        };

        let mut buffer = [0u8; 4096];
        let mut read_data_bytes_return = 0u32;

        unsafe {
            let success = DeviceIoControl(
                volume_handle,
                FSCTL_READ_USN_JOURNAL,
                Some(&read_data as *const _ as *mut _),
                size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(&mut buffer as *const _ as *mut _),
                4096,
                Some(&mut read_data_bytes_return),
                None,
            );

            println!("Read USN journal result = {:?}", success);

            if !success.as_bool() {
                let error = Error::from_win32();
                println!("last error = {:?}", error);
            }

            println!("read data bytes return: {:?}", read_data_bytes_return);

            let mut offset = 8; // sizeof(USN)

            while offset < read_data_bytes_return {
                let record;
                let record_length;

                let record_raw = transmute::<*const u8, *const USN_RECORD_UNION>(
                    buffer[offset as usize..].as_ptr(),
                );
                let header = &(*record_raw).Header;

                if header.RecordLength == 0 || header.MajorVersion != 2 {
                    println!("Unsupported major version: {}", header.MajorVersion);
                    break;
                }

                record_length = header.RecordLength;
                record = &(*record_raw).V2;

                println!("{:?}", record);

                offset += record_length;
            }
        };
    }
}

fn get_usn_file_name(record: USN_RECORD_V2) -> String {
    // FileNameLength is the length of the name of the file or directory associated with this record, in bytes.
    //  but the USN_RECORD_V2.FileName is u16, so we have to do the division to get the real UTF-16 length
    // The file name length does not count the terminating null character
    let file_name_len = record.FileNameLength as usize / std::mem::size_of::<u16>();

    if file_name_len > 0 {
        let file_name_u16 =
            unsafe { slice::from_raw_parts(record.FileName.as_ptr() as *const u16, file_name_len) };
        let file_name = OsString::from_wide(file_name_u16)
            .to_string_lossy()
            .into_owned();

        return file_name;
    }

    return String::new();
}

fn get_file_path(volume_handle: &HANDLE, file_id: u64) -> anyhow::Result<PathBuf> {
    let file_id_desc = FILE_ID_DESCRIPTOR {
        Type: FileSystem::ExtendedFileIdType,
        dwSize: size_of::<FileSystem::FILE_ID_DESCRIPTOR>() as u32,
        Anonymous: FileSystem::FILE_ID_DESCRIPTOR_0 {
            FileId: file_id.try_into()?,
        },
    };

    unsafe {
        let file_handle = FileSystem::OpenFileById(
            *volume_handle,
            &file_id_desc,
            FileSystem::FILE_GENERIC_READ.0,
            FileSystem::FILE_SHARE_READ
                | FileSystem::FILE_SHARE_WRITE
                | FileSystem::FILE_SHARE_DELETE,
            None,
            FILE_FLAGS_AND_ATTRIBUTES::default(),
        )?;

        let info_buffer_size = size_of::<FileSystem::FILE_NAME_INFO>()
            + (Foundation::MAX_PATH as usize) * size_of::<u16>();
        let mut info_buffer = vec![0u8; info_buffer_size];
        let info_result = FileSystem::GetFileInformationByHandleEx(
            file_handle,
            FileSystem::FileNameInfo,
            &mut *info_buffer as *mut _ as *mut c_void,
            info_buffer_size as u32,
        );

        Foundation::CloseHandle(file_handle);

        if info_result.as_bool() {
            let (_, body, _) = info_buffer.align_to::<FileSystem::FILE_NAME_INFO>();
            let info = &body[0];
            let name_len = info.FileNameLength as usize / size_of::<u16>();
            let name_u16 =
                std::slice::from_raw_parts(info.FileName.as_ptr() as *const u16, name_len);
            let path = PathBuf::from(std::ffi::OsString::from_wide(name_u16));
            return Ok(path);
        } else {
            Err(Error::from_win32().into())
        }
    }
}
