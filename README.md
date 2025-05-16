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

**General Syntax:**

```powershell
.\usn-parser.exe <COMMAND> <VOLUME_LETTER> [OPTIONS]
```
Or if installed via cargo:
```powershell
usn-parser <COMMAND> <VOLUME_LETTER> [OPTIONS]
```

### Commands

####  монитор (Monitor) 📡
Monitor real-time USN journal changes.

**Syntax:**
```powershell
usn-parser monitor <VOLUME_LETTER> [OPTIONS]
```

**Example:** Monitor drive `C` for all changes:
```powershell
usn-parser monitor C
```
Monitor drive `D` for changes to files containing "report" in their name:
```powershell
usn-parser monitor D -f "*report*" --file-only
```

#### поиск (Search) 🔎
Search the Master File Table.

**Syntax:**
```powershell
usn-parser search <VOLUME_LETTER> [OPTIONS]
```

**Example:** Search drive `C` for all directory entries:
```powershell
usn-parser search C --dir-only
```
Search drive `E` for files matching `*.docx`:
```powershell
usn-parser search E -f "*.docx" --file-only
```

#### читать (Read) 📖
Read history USN journal entries.

**Syntax:**
```powershell
usn-parser read <VOLUME_LETTER> [OPTIONS]
```

**Example:** Read all USN journal entries from drive `C`:
```powershell
usn-parser read C
```
Read USN journal entries from drive `F` related to directories with "backup" in their name:
```powershell
usn-parser read F --filter "*backup*" --dir-only
```

### Options

*   `<VOLUME_LETTER>`: The volume name to target (e.g., `C`, `D`). (Required)
*   `-f, --filter <KEYWORD>`: Filter results by keyword. Wildcards (`*`, `?`) are permitted.
*   `--file-only`: Only display file entries.
*   `--dir-only`: Only display directory entries.

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