#  Rust USN Parser 🦀

A command-line utility 💻 for parsing NTFS/ReFS USN (Update Sequence Number) Journals and searching the MFT (Master File Table) on Windows systems.

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

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/wangfu91/usn-parser-rs.git
    cd usn-parser-rs
    ```

2.  **Build the project:**
    ```bash
    cargo build --release
    ```
    The executable will be located at `target/release/usn-parser.exe`.

Alternatively, you can install directly using `cargo install` if the crate is published to crates.io:
```bash
cargo install usn-parser
```

## 🛠️ Usage

The utility requires administrator privileges to run.

To see available commands and global options:
```powershell
usn-parser --help
```
To get help for a specific command:
```powershell
usn-parser <COMMAND> --help
```

### Examples

#### Monitor 📡
Monitor real-time USN journal changes.

*   Monitor drive `C` for all real-time USN records:
    ```powershell
    usn-parser monitor C
    ```
*   Monitor drive `D` for real-time USN records, filtering for `.txt` files whose names start with "log":
    ```powershell
    usn-parser monitor -f "log*.txt" --file-only D
    ```

#### Search 🔎
Search the Master File Table.

*   Search the Master File Table of volume `D`, printing out all files with the extension `.xlsx`:
    ```powershell
    usn-parser search -f "*.xlsx" --file-only D
    ```
*   Search the Master File Table of volume `C` for all directory entries:
    ```powershell
    usn-parser search --dir-only C
    ```

#### Read 📖
Read history USN journal entries.

*   Print out the change history for a file named `ImportantDocument.docx` from the USN journal of volume `D`:
    ```powershell
    usn-parser read -f "ImportantDocument.docx" D
    ```
*   Read all USN journal entries from drive `F` related to directories with "archive" in their name:
    ```powershell
    usn-parser read --filter "*archive*" --dir-only F
    ```

## 🏗️ Building from Source

1.  Ensure you have Rust and Cargo installed.
2.  Clone the repository:
    ```bash
    git clone https://github.com/wangfu91/usn-parser-rs.git
    cd usn-parser-rs
    ```
3.  Build the project:
    ```bash
    cargo build
    ```
    For a release build (optimized):
    ```bash
    cargo build --release
    ```
    The executable will be in the `target/debug` or `target/release` directory.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request or open an issue.

## 📜 License

This project is licensed under the terms of the [MIT LICENSE](LICENSE).