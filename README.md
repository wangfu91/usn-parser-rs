#  usn-parser

A command-line utility 💻 for parsing NTFS/ReFS USN Change Journal and searching the NTFS MFT on Windows systems.

[![Rust](https://img.shields.io/badge/rust-stable-blue.svg)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## ✨ Features

*   **Monitor Real-time Changes**: Keep an eye on USN journal entries as they happen. ⏱️
*   **Search MFT**: Efficiently search the Master File Table for specific entries. 🔍
*   **Read USN Journal History**: Access and analyze historical USN journal data. 📜
*   **Flexible Filtering**:
    *   Filter by keyword (wildcards supported).
    *   Show only files 📄 or only directories 📁.

## 🚀 Getting Started

### Prerequisites

*   Rust programming language and Cargo package manager installed. You can get them from [rustup.rs](https://rustup.rs/).
*   Administrator privileges are required to access USN journals and the MFT.

### Installation

The crate has been published to [crates.io](https://crates.io/crates/usn-parser), you can install it using Cargo:
```bash
cargo install usn-parser
```

## 🛠️ Usage

```powershell
Usage: usn-parser.exe <COMMAND>

Commands:
  monitor  Monitor real-time USN journal changes
  search   Search the Master File Table
  read     Read history USN journal entries
  help     Print this message or the help of the given subcommand(s)

Options:
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Examples

#### Monitor 📡
Monitor real-time USN journal changes.

*   Monitor drive `C` for all real-time USN records:
    ```powershell
    usn-parser monitor C
    ```
*   Monitor drive `C` for real-time USN records, filtering for log files with the name prefix `app`:
    ```powershell
    usn-parser monitor C -f "app*.log" --file-only
    ```

#### Search 🔎
Search the Master File Table.

*   Search the Master File Table of volume `C`, printing out all files with the extension `.xlsx`:
    ```powershell
    usn-parser search C -f "*.xlsx" --file-only
    ```
*   Search the Master File Table of volume `D` for all directory entries:
    ```powershell
    usn-parser search D --dir-only
    ```

#### Read 📖
Read history USN journal entries.

*   Print out the change history for a file named `report.docx` from the USN journal of volume `D`:
    ```powershell
    usn-parser read D -f "report.docx"
    ```
*   Read all USN journal entries from drive `F` related to directories with "archive" in their name:
    ```powershell
    usn-parser read F --filter "*archive*" --dir-only
    ```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request or open an issue.

## 📜 License

This project is licensed under the terms of the [MIT LICENSE](LICENSE).