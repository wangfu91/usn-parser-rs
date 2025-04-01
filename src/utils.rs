use std::{
    ffi::{OsString, c_void},
    os::windows::ffi::OsStringExt,
    path::PathBuf,
};

use anyhow::Context;
use windows::{
    Win32::{
        Foundation::{self, HANDLE},
        Storage::FileSystem::{
            self, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_ID_DESCRIPTOR,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    },
    core::HSTRING,
};

pub fn get_volume_handle(volume: &str) -> anyhow::Result<HANDLE> {
    let volume_root = format!(r"\\.\{}", volume.trim_end_matches('\\'));

    let volume_handle = unsafe {
        CreateFileW(
            &HSTRING::from(&volume_root),
            FILE_GENERIC_READ.0,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            None,
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES::default(),
            None,
        )
        .with_context(|| {
            format!(
                "CreateFileW failed, volume_root={}, error={:?}",
                volume_root,
                Foundation::GetLastError()
            )
        })?
    };

    Ok(volume_handle)
}

pub fn file_id_to_path(volume_handle: HANDLE, file_id: u64) -> anyhow::Result<PathBuf> {
    let file_id_desc = FILE_ID_DESCRIPTOR {
        Type: FileSystem::FileIdType,
        dwSize: size_of::<FileSystem::FILE_ID_DESCRIPTOR>() as u32,
        Anonymous: FileSystem::FILE_ID_DESCRIPTOR_0 {
            FileId: file_id.try_into()?,
        },
    };

    let file_handle = unsafe {
        FileSystem::OpenFileById(
            volume_handle,
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
    let name_u16 = unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), name_len) };
    let path = PathBuf::from(OsString::from_wide(name_u16));
    Ok(path)
}
