use std::{ffi::c_void, mem::size_of};

use windows::Win32::{
    Foundation::{ERROR_HANDLE_EOF, HANDLE},
    System::{
        Ioctl::{
            CREATE_USN_JOURNAL_DATA, DELETE_USN_JOURNAL_DATA, FSCTL_CREATE_USN_JOURNAL,
            FSCTL_DELETE_USN_JOURNAL, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL,
            READ_USN_JOURNAL_DATA_V0, USN_DELETE_FLAGS, USN_DELETE_FLAG_DELETE,
            USN_DELETE_FLAG_NOTIFY, USN_JOURNAL_DATA_V0, USN_RECORD_V2,
        },
        IO::DeviceIoControl,
    },
};

use crate::{usn_entry::UsnEntry, Usn, DEFAULT_BUFFER_SIZE};

pub struct UsnJournal {
    pub volume_handle: HANDLE,
    pub journal_id: u64,
    buffer: Vec<u8>,
    bytes_read: u32,
    offset: u32,
    next_start_usn: Usn,
    reason_mask: u32,
    return_only_on_close: u32,
    timeout: u64,
    bytes_to_wait_for: u64,
}

pub struct UsnJournalEnumOptions {
    pub start_usn: Usn,
    pub reason_mask: u32,
    pub only_on_close: bool,
    pub timeout: u64,
    pub wait_for_more: bool,
    pub buffer_size: usize,
}

impl Default for UsnJournalEnumOptions {
    fn default() -> Self {
        UsnJournalEnumOptions {
            start_usn: 0,
            reason_mask: 0xFFFFFFFF,
            only_on_close: false,
            timeout: 0,
            wait_for_more: false,
            buffer_size: DEFAULT_BUFFER_SIZE,
        }
    }
}

impl UsnJournal {
    pub fn new(volume_handle: HANDLE, journal_id: u64) -> Self {
        Self {
            volume_handle,
            journal_id,
            buffer: vec![0u8; DEFAULT_BUFFER_SIZE],
            bytes_read: 0,
            offset: 0,
            next_start_usn: 0,
            reason_mask: 0xFFFFFFFF,
            return_only_on_close: 0,
            timeout: 0,
            bytes_to_wait_for: 1,
        }
    }

    pub fn new_with_options(
        volume_handle: HANDLE,
        journal_id: u64,
        options: UsnJournalEnumOptions,
    ) -> Self {
        Self {
            volume_handle,
            journal_id,
            buffer: vec![0u8; options.buffer_size],
            bytes_read: 0,
            offset: 0,
            next_start_usn: options.start_usn,
            reason_mask: options.reason_mask,
            return_only_on_close: options.only_on_close as u32,
            timeout: options.timeout,
            bytes_to_wait_for: options.wait_for_more as u64,
        }
    }

    fn get_data(&mut self) -> anyhow::Result<bool> {
        let read_data = READ_USN_JOURNAL_DATA_V0 {
            StartUsn: self.next_start_usn,
            ReasonMask: self.reason_mask,
            ReturnOnlyOnClose: self.return_only_on_close,
            Timeout: self.timeout,
            BytesToWaitFor: self.bytes_to_wait_for,
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

pub fn query(volume_handle: HANDLE) -> anyhow::Result<USN_JOURNAL_DATA_V0> {
    let journal_data = USN_JOURNAL_DATA_V0::default();
    let bytes_return = 0u32;

    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/fileio/using-the-change-journal-identifier
        // To obtain the identifier of the current change journal on a specified volume,
        // use the FSCTL_QUERY_USN_JOURNAL control code.
        // To perform this and all other change journal operations,
        // you must have system administrator privileges.
        // That is, you must be a member of the Administrators group.
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

pub fn create_or_update(
    volume_handle: HANDLE,
    max_size: u64,
    allocation_delta: u64,
) -> anyhow::Result<()> {
    let create_data = CREATE_USN_JOURNAL_DATA {
        MaximumSize: max_size,
        AllocationDelta: allocation_delta,
    };

    unsafe {
        // https://learn.microsoft.com/en-us/windows/win32/api/winioctl/ni-winioctl-fsctl_create_usn_journal
        // FSCTL_CREATE_USN_JOURNAL
        // Creates an update sequence number (USN) change journal stream on a target volume, or modifies an existing change journal stream.
        DeviceIoControl(
            volume_handle,
            FSCTL_CREATE_USN_JOURNAL,
            Some(&create_data as *const _ as *mut _),
            size_of::<CREATE_USN_JOURNAL_DATA>() as u32,
            None,
            0,
            None,
            None,
        )
    }?;

    println!("Created USN journal successfully.");

    Ok(())
}

pub fn delete(volume_handle: HANDLE, journal_id: u64) -> anyhow::Result<()> {
    let delete_flags: USN_DELETE_FLAGS = USN_DELETE_FLAG_DELETE | USN_DELETE_FLAG_NOTIFY;
    let delete_data = DELETE_USN_JOURNAL_DATA {
        UsnJournalID: journal_id,
        DeleteFlags: delete_flags,
    };

    unsafe {
        DeviceIoControl(
            volume_handle,
            FSCTL_DELETE_USN_JOURNAL,
            Some(&delete_data as *const _ as *mut _),
            size_of::<DELETE_USN_JOURNAL_DATA>() as u32,
            None,
            0,
            None,
            None,
        )
    }?;

    println!("Deleted USN journal successfully.");

    Ok(())
}

#[cfg(test)]
mod tests {
    use anyhow::Ok;

    #[test]
    fn query_usn_journal_test() -> anyhow::Result<()> {
        let volume_letter = "E:\\";
        let volume_handle = crate::utils::get_volume_handle(volume_letter)?;
        let _data = super::query(volume_handle)?;

        Ok(())
    }

    #[test]
    fn delete_usn_journal_test() -> anyhow::Result<()> {
        let volume_letter = "E:\\";
        let volume_handle = crate::utils::get_volume_handle(volume_letter)?;
        let data = super::query(volume_handle)?;
        super::delete(volume_handle, data.UsnJournalID)?;

        Ok(())
    }

    #[test]
    fn create_usn_journal_test() -> anyhow::Result<()> {
        let volume_letter = "E:\\";
        let volume_handle = crate::utils::get_volume_handle(volume_letter)?;
        super::create_or_update(volume_handle, 1024 * 1024 * 1024, 1024 * 1024)?;

        Ok(())
    }

    #[test]
    fn usn_journal_iter_test() -> anyhow::Result<()> {
        let volume_letter = "E:\\";
        let volume_handle = crate::utils::get_volume_handle(volume_letter)?;
        let journal_data = super::query(volume_handle)?;
        let option = super::UsnJournalEnumOptions::default();
        let usn_journal =
            super::UsnJournal::new_with_options(volume_handle, journal_data.UsnJournalID, option);
        let mut previous_usn = -1i64;
        for entry in usn_journal {
            // Check if the USN entry is valid
            assert!(entry.usn >= 0, "USN is not valid");
            assert!(entry.usn > previous_usn, "USN entries are not in order");
            assert!(entry.fid > 0, "File ID is not valid");
            assert!(!entry.file_name.is_empty(), "File name is not valid");
            assert!(entry.parent_fid > 0, "Parent File ID is not valid");
            assert!(entry.reason > 0, "Reason is not valid");
            assert!(entry.file_attributes.0 > 0, "File attributes are not valid");
            assert!(entry.time > std::time::UNIX_EPOCH, "Time is not valid");

            previous_usn = entry.usn;
        }

        Ok(())
    }
}
