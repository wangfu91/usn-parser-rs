use std::{
    ffi::{c_void, OsString},
    os::windows::ffi::OsStringExt,
    path::PathBuf,
};

use anyhow::Context;
use chrono::{DateTime, TimeZone, Utc};
use windows::Win32::{
    Foundation::{FILETIME, SYSTEMTIME},
    System::Time::FileTimeToSystemTime,
};
use windows::{
    core::HSTRING,
    Win32::{
        Foundation::{self, HANDLE},
        Storage::FileSystem::{
            self, CreateFileW, FILE_FLAGS_AND_ATTRIBUTES, FILE_GENERIC_READ, FILE_ID_DESCRIPTOR,
            FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING,
        },
    },
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
/// to a chrono DateTime<Utc>.
pub fn filetime_to_datetime(filetime: i64) -> anyhow::Result<DateTime<Utc>> {
    // Convert i64 to FILETIME struct
    let ft = FILETIME {
        dwLowDateTime: (filetime & 0xFFFFFFFF) as u32,
        dwHighDateTime: (filetime >> 32) as u32,
    };

    // Convert FILETIME to SYSTEMTIME
    let mut st = SYSTEMTIME::default();
    if let Err(err) = unsafe { FileTimeToSystemTime(&ft, &mut st) } {
        return Err(err.into());
    }

    // Convert SYSTEMTIME to chrono::DateTime
    let dt = Utc
        .with_ymd_and_hms(
            st.wYear as i32,
            st.wMonth as u32,
            st.wDay as u32,
            st.wHour as u32,
            st.wMinute as u32,
            st.wSecond as u32,
        )
        .single()
        .context("Failed to convert SYSTEMTIME to DateTime")?;

    Ok(dt)
}

#[cfg(test)]
mod tests {

    use super::*;
    use anyhow::anyhow;
    use chrono::{Datelike, Timelike};
    use windows::Win32::System::Time::SystemTimeToFileTime;

    #[test]
    fn filetime_to_datetime_test() -> anyhow::Result<()> {
        // Test with the Unix Epoch (January 1, 1970 00:00:00 UTC)
        // 116444736000000000 = Windows FILETIME for Unix Epoch
        let unix_epoch_filetime: i64 = 116444736000000000;
        let unix_epoch_datetime = filetime_to_datetime(unix_epoch_filetime)?;

        assert_eq!(unix_epoch_datetime.year(), 1970);
        assert_eq!(unix_epoch_datetime.month(), 1);
        assert_eq!(unix_epoch_datetime.day(), 1);
        assert_eq!(unix_epoch_datetime.hour(), 0);
        assert_eq!(unix_epoch_datetime.minute(), 0);
        assert_eq!(unix_epoch_datetime.second(), 0);

        // Instead of trying to create specific FILETIME values, which is error-prone,
        // let's create a datetime and then convert it back from the FILETIME
        // to verify the function works correctly.

        // First, let's create a simple datetime for testing
        let test_datetime = unsafe {
            let st = SYSTEMTIME {
                wYear: 2020,
                wMonth: 1,
                wDay: 1,
                wHour: 0,
                wMinute: 0,
                wSecond: 0,
                wMilliseconds: 0,
                ..Default::default()
            };

            // Convert SYSTEMTIME to FILETIME
            let mut ft = FILETIME::default();
            let result = SystemTimeToFileTime(&st, &mut ft);
            if result.is_err() {
                return Err(anyhow!("Failed to convert SYSTEMTIME to FILETIME"));
            }

            // Convert FILETIME to i64
            let filetime_i64 = ((ft.dwHighDateTime as i64) << 32) | (ft.dwLowDateTime as i64);

            // Now convert back to DateTime for testing
            filetime_to_datetime(filetime_i64)?
        };

        // Verify the converted DateTime has the expected values
        assert_eq!(test_datetime.year(), 2020);
        assert_eq!(test_datetime.month(), 1);
        assert_eq!(test_datetime.day(), 1);
        assert_eq!(test_datetime.hour(), 0);
        assert_eq!(test_datetime.minute(), 0);
        assert_eq!(test_datetime.second(), 0);

        // Let's test another date
        let test_datetime2 = unsafe {
            let st = SYSTEMTIME {
                wYear: 2023,
                wMonth: 7,
                wDay: 15,
                wHour: 12,
                wMinute: 30,
                wSecond: 45,
                wMilliseconds: 0,
                ..Default::default()
            };

            // Convert SYSTEMTIME to FILETIME
            let mut ft = FILETIME::default();
            let result = SystemTimeToFileTime(&st, &mut ft);
            if result.is_err() {
                return Err(anyhow!("Failed to convert SYSTEMTIME to FILETIME"));
            }

            // Convert FILETIME to i64
            let filetime_i64 = ((ft.dwHighDateTime as i64) << 32) | (ft.dwLowDateTime as i64);

            // Now convert back to DateTime for testing
            filetime_to_datetime(filetime_i64)?
        };

        // Verify the converted DateTime has the expected values
        assert_eq!(test_datetime2.year(), 2023);
        assert_eq!(test_datetime2.month(), 7);
        assert_eq!(test_datetime2.day(), 15);
        assert_eq!(test_datetime2.hour(), 12);
        assert_eq!(test_datetime2.minute(), 30);
        assert_eq!(test_datetime2.second(), 45);

        Ok(())
    }
}
