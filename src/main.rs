use std::{
    default::Default,
    ffi::{c_void, OsString},
    fmt::Result,
    mem,
    os::windows::prelude::OsStringExt,
    ptr,
};

use bindings::Windows::Win32::{
    Foundation::HANDLE,
    Security::SECURITY_ATTRIBUTES,
    Storage::FileSystem::{
        CreateFileW, FILE_ACCESS_FLAGS, FILE_CREATION_DISPOSITION, FILE_FLAGS_AND_ATTRIBUTES,
        FILE_FLAG_BACKUP_SEMANTICS, FILE_GENERIC_READ, FILE_SHARE_MODE, FILE_SHARE_READ,
        FILE_SHARE_WRITE, OPEN_EXISTING, READ_USN_JOURNAL_DATA_V0, USN_JOURNAL_DATA_V0,
        USN_RECORD_V2,
    },
    System::{
        Diagnostics::Debug::GetLastError,
        SystemServices::{DeviceIoControl, FSCTL_QUERY_USN_JOURNAL, FSCTL_READ_USN_JOURNAL},
    },
};

use byteorder::{ByteOrder, LittleEndian};

fn main() {
    // https://microsoft.github.io/windows-docs-rs/doc/bindings/Windows/Win32/System/Diagnostics/Debug/struct.WIN32_ERROR.html
    unsafe {
        let hVol = CreateFileW(
            "\\\\.\\C:",
            FILE_GENERIC_READ,
            FILE_SHARE_READ | FILE_SHARE_WRITE,
            &SECURITY_ATTRIBUTES::default(),
            OPEN_EXISTING,
            FILE_FLAGS_AND_ATTRIBUTES(0),
            HANDLE(0),
        );

        println!("hVol = {:?}", hVol);
        let error = GetLastError();
        println!("last error: {:?}", error);

        let journalData = USN_JOURNAL_DATA_V0::default();
        let queryJournalBytesReturn = 0u32;
        let mut nextUsn = 0i64;
        let mut journalId = 0u64;

        unsafe {
            let result = DeviceIoControl(
                hVol,
                FSCTL_QUERY_USN_JOURNAL,
                ptr::null_mut(),
                0,
                &journalData as *const _ as *mut _,
                mem::size_of::<USN_JOURNAL_DATA_V0>() as u32,
                &queryJournalBytesReturn as *const _ as *mut _,
                ptr::null_mut(),
            );

            println!("Query change journal result: {:?}", result);

            let error = GetLastError();
            println!("last error: {:?}", error);

            println!("Journal data: {:?}", journalData);
            println!("bytesReturn: {:?}", queryJournalBytesReturn);

            nextUsn = journalData.NextUsn;
            journalId = journalData.UsnJournalID;
        };

        // Read USN journal

        let mut readData = READ_USN_JOURNAL_DATA_V0::default();
        readData.StartUsn = nextUsn;
        readData.ReasonMask = 0xFFFFFFFF;
        readData.ReturnOnlyOnClose = 0;
        readData.Timeout = 0;
        readData.BytesToWaitFor = 1;
        readData.UsnJournalID = journalId;

        let buffer = [0u8; 4096];

        let readDataBytesReturn = 0u32;
        unsafe {
            let result = DeviceIoControl(
                hVol,
                FSCTL_READ_USN_JOURNAL,
                &readData as *const _ as *mut _,
                mem::size_of::<READ_USN_JOURNAL_DATA_V0>() as u32,
                &buffer as *const _ as *mut _,
                4096,
                &readDataBytesReturn as *const _ as *mut _,
                ptr::null_mut(),
            );

            let data_buffer = &buffer[..readDataBytesReturn as usize];
            println!("data_buffer: {:?}", data_buffer);

            println!("Read USN journal result = {:?}", result);

            let error = GetLastError();
            println!("last error = {:?}", error);

            println!("read data bytes return: {:?}", readDataBytesReturn);

            // USN_RECORD.
            let mut offset = 8u32 as usize;
            loop {
                println!("new loop, offset = {:?}", offset);
                if offset >= readDataBytesReturn as usize{
                    break;
                }

                let usn_record = parse_to_usn_record(&data_buffer[offset..]);
                println!("usn record: {:#?}", &usn_record);
                offset += usn_record.RecordLength as usize;                    
            }
        }
    }
}

fn parse_to_usn_record(input: &[u8]) -> USN_RECORD_V2 {
    let record_length = LittleEndian::read_u32(input);
    let major_version = LittleEndian::read_u16(&input[4..]);
    let minor_version = LittleEndian::read_u16(&input[6..]);

    let frn = LittleEndian::read_u64(&input[8..]);
    let parent_frn = LittleEndian::read_u64(&input[16..]);
    let usn = LittleEndian::read_i64(&input[24..]);
    let timestamp = LittleEndian::read_i64(&input[32..]);
    let reason = LittleEndian::read_u32(&input[40..]);
    let source_info = LittleEndian::read_u32(&input[44..]);
    let security_id = LittleEndian::read_u32(&input[48..]);
    let file_attribute = LittleEndian::read_u32(&input[52..]);
    let file_name_length = LittleEndian::read_u16(&input[56..]);
    let file_name_offset = LittleEndian::read_u16(&input[58..]);

    let mut v: Vec<u16> = vec![];
    for c in input[60..].chunks(2) {
        v.push(LittleEndian::read_u16(c));
    }

    let file_name = OsString::from_wide(&v[..]).into_string().unwrap();
    println!("file_name = {:?}", &file_name);

    USN_RECORD_V2 {
        RecordLength: record_length,
        MajorVersion: major_version,
        MinorVersion: minor_version,
        FileReferenceNumber: frn,
        ParentFileReferenceNumber: parent_frn,
        Usn: usn,
        TimeStamp: timestamp,
        Reason: reason,
        SourceInfo: source_info,
        SecurityId: security_id,
        FileAttributes: file_attribute,
        FileNameLength: file_name_length,
        FileNameOffset: file_name_offset,
        FileName: [0u16; 1],
    }
}
