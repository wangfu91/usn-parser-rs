use std::{ffi::c_void, mem::size_of};

use windows::Win32::{
    Foundation::{ERROR_HANDLE_EOF, HANDLE},
    System::{
        IO::DeviceIoControl,
        Ioctl::{
            FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL, READ_USN_JOURNAL_DATA_V0,
            USN_JOURNAL_DATA_V0, USN_RECORD_V2,
        },
    },
};

use crate::usn_entry::UsnEntry;

type Usn = i64;

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

pub struct UsnJournal {
    pub volume_handle: HANDLE,
    pub journal_id: u64,
    buffer: [u8; 64 * 1024],
    bytes_read: u32,
    offset: u32,
    next_start_usn: Usn,
}

impl UsnJournal {
    pub fn new(volume_handle: HANDLE, journal_id: u64, start_usn: Usn) -> Self {
        Self {
            volume_handle,
            journal_id,
            buffer: [0u8; 64 * 1024],
            bytes_read: 0,
            offset: 0,
            next_start_usn: start_usn,
        }
    }

    fn get_data(&mut self) -> anyhow::Result<bool> {
        let read_data = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: self.next_start_usn,
            ReasonMask: 0xFFFFFFFF,
            ReturnOnlyOnClose: 0,
            Timeout: 0,
            BytesToWaitFor: 1,
            UsnJournalID: self.journal_id,
        };

        if let Err(err) = unsafe {
            DeviceIoControl(
                self.volume_handle,
                FSCTL_READ_USN_JOURNAL,
                Some(&read_data as *const _ as *mut _),
                size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                Some(self.buffer.as_mut_ptr() as *mut c_void),
                self.buffer.len() as u32,
                Some(&mut self.bytes_read),
                None,
            )
        } {
            if err.code() == ERROR_HANDLE_EOF.into() {
                return Ok(false);
            }

            println!("Error reading USN data: {}", err);
            return Err(err.into());
        }

        Ok(true)
    }

    fn find_next_entry(&mut self) -> anyhow::Result<Option<&USN_RECORD_V2>> {
        if self.offset < self.bytes_read {
            let record = unsafe {
                &*(self.buffer.as_ptr().offset(self.offset as isize) as *const USN_RECORD_V2)
            };
            self.offset += record.RecordLength;
            return Ok(Some(record));
        }

        // We need to read more data
        if self.get_data()? {
            // https://learn.microsoft.com/en-us/windows/win32/fileio/walking-a-buffer-of-change-journal-records
            // The USN returned as the first item in the output buffer is the USN of the next record number to be retrieved.
            // Use this value to continue reading records from the end boundary forward.
            self.next_start_usn = unsafe { std::ptr::read(self.buffer.as_ptr() as *const Usn) };
            self.offset = std::mem::size_of::<Usn>() as u32;

            if self.offset < self.bytes_read {
                let record = unsafe {
                    &*(self.buffer.as_ptr().offset(self.offset as isize) as *const USN_RECORD_V2)
                };
                self.offset += record.RecordLength;
                return Ok(Some(record));
            }
        }

        // EOF, no more data to read
        Ok(None)
    }
}

impl Iterator for UsnJournal {
    type Item = UsnEntry;

    fn next(&mut self) -> Option<Self::Item> {
        match self.find_next_entry() {
            Ok(Some(record)) => Some(UsnEntry::new(record)),
            Ok(None) => None,
            Err(err) => {
                println!("Error finding next USN entry: {}", err);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn iter_test() {}
}
