use std::{
    ffi::{OsString, c_void},
    os::windows::ffi::OsStringExt,
    path::PathBuf,
};

use anyhow::Context;
use chrono::{DateTime, FixedOffset, TimeZone, Utc};
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

pub fn file_id_to_path(
    volume_handle: HANDLE,
    drive_letter: char,
    file_id: u64,
) -> anyhow::Result<PathBuf> {
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
    let info = body.first().context("Failed to read FILE_NAME_INFO")?;
    let name_len = info.FileNameLength as usize / size_of::<u16>();
    let name_u16 = unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), name_len) };
    let sub_path = OsString::from_wide(name_u16);
    // Only convert to uppercase if it's lowercase
    let drive_char = if drive_letter.is_ascii_lowercase() {
        drive_letter.to_ascii_uppercase()
    } else {
        drive_letter
    };

    // Create the full path directly with a single allocation
    let mut full_path = PathBuf::new();
    full_path.push(format!("{}:\\", drive_char));
    full_path.push(sub_path);
    Ok(full_path)
}

/// Converts a Windows FILETIME (64-bit value, number of 100-nanosecond intervals since January 1, 1601 UTC)
/// to a chrono DateTime<FixedOffset>.
pub fn filetime_to_datetime(filetime: i64) -> DateTime<FixedOffset> {
    // Number of 100-nanosecond intervals between 1601-01-01 and 1970-01-01
    const WINDOWS_TO_UNIX_EPOCH: i64 = 116444736000000000;

    // Convert to microseconds (10 100-nanosecond intervals = 1 microsecond)
    let unix_time_micros = (filetime - WINDOWS_TO_UNIX_EPOCH) / 10;

    // Convert to seconds and nanoseconds
    let unix_time_secs = unix_time_micros / 1_000_000;
    let unix_time_nsecs = ((unix_time_micros % 1_000_000) * 1000) as u32;

    // Create DateTime from timestamp with UTC timezone
    Utc.timestamp_opt(unix_time_secs, unix_time_nsecs)
        .single() // Get the single valid timestamp or None
        .unwrap_or_else(|| Utc.timestamp_nanos(0)) // Fallback to epoch if invalid
        .with_timezone(&FixedOffset::east_opt(0).unwrap()) // Convert to FixedOffset
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike, Utc};

    #[test]
    fn test_filetime_to_datetime() {
        // Test case 1: A known Windows FILETIME (2020-01-01 12:00:00 UTC)
        // 132224440000000000 = January 1, 2020 12:00:00 UTC in FILETIME
        let filetime = 132224440000000000;
        let expected = Utc
            .with_ymd_and_hms(2020, 1, 1, 12, 0, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let result = filetime_to_datetime(filetime);
        assert_eq!(result, expected);

        // Test case 2: Windows FILETIME epoch (1601-01-01 00:00:00 UTC)
        let filetime = 0;
        let expected = Utc
            .with_ymd_and_hms(1601, 1, 1, 0, 0, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let result = filetime_to_datetime(filetime);
        assert_eq!(result, expected);

        // Test case 3: Unix epoch (1970-01-01 00:00:00 UTC)
        let filetime = 116444736000000000; // Windows to Unix epoch constant
        let expected = Utc
            .with_ymd_and_hms(1970, 1, 1, 0, 0, 0)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let result = filetime_to_datetime(filetime);
        assert_eq!(result, expected);

        // Test case 4: Precision test with microseconds
        // 132224440001234560 = January 1, 2020 12:00:00.123456 UTC in FILETIME
        let filetime = 132224440001234560;
        let expected = Utc
            .with_ymd_and_hms(2020, 1, 1, 12, 0, 0)
            .unwrap()
            .with_nanosecond(123456000)
            .unwrap()
            .with_timezone(&FixedOffset::east_opt(0).unwrap());
        let result = filetime_to_datetime(filetime);
        assert_eq!(result, expected);
    }
}
