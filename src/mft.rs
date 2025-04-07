use windows::Win32::{
    Foundation::{ERROR_HANDLE_EOF, HANDLE},
    System::{
        IO::DeviceIoControl,
        Ioctl::{self, USN_RECORD_V2},
    },
};

use crate::usn_entry::UsnEntry;

type Usn = i64;

pub struct Mft {
    volume_handle: HANDLE,
    high_usn: Usn,
    buffer: [u8; 64 * 1024],
    bytes_read: u32,
    offset: u32,
    next_start_fid: u64,
}

impl Mft {
    pub fn new(volume_handle: HANDLE, high_usn: Usn) -> Self {
        Mft {
            volume_handle,
            high_usn,
            buffer: [0u8; 64 * 1024],
            bytes_read: 0,
            offset: 0,
            next_start_fid: 0,
        }
    }

    fn get_data(&mut self) -> anyhow::Result<bool> {
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
                return Ok(false);
            }

            println!("Error reading MFT data: {}", err);
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
            // Each call to FSCTL_ENUM_USN_DATA retrieves the starting point for the subsequent call as the first entry in the output buffer.
            self.next_start_fid = unsafe { std::ptr::read(self.buffer.as_ptr() as *const u64) };
            self.offset = size_of::<u64>() as u32;
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

impl Iterator for Mft {
    type Item = UsnEntry;

    fn next(&mut self) -> Option<Self::Item> {
        match self.find_next_entry() {
            Ok(Some(record)) => Some(UsnEntry::new(record)),
            Ok(None) => None,
            Err(err) => {
                println!("Error finding next MFT entry: {}", err);
                None
            }
        }
    }
}
