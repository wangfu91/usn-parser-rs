#  usn-parser

A command-line utility for searching the NTFS MFT and parsing NTFS/ReFS USN Change Journal on Windows.

[![Crates.io](https://img.shields.io/crates/v/usn-parser.svg)](https://crates.io/crates/usn-parser)
[![Downloads](https://img.shields.io/crates/d/usn-parser.svg)](https://crates.io/crates/usn-parser)
[![License](https://img.shields.io/badge/license-MIT-green.svg)](LICENSE)

## ✨ Features

* 👀 **Monitor Real-time Changes**: Keep an eye on USN journal entries as they happen. 
* 🔍 **Search MFT**: Efficiently search the Master File Table for specific entries.
* 📖 **Read Journal Change History**: Access and analyze historical USN journal data.
* 🔽 **Flexible Filtering**:
    *   Filter by keyword (wildcards supported).
    *   Show only files or only directories.

## 📥 Installation

The crate has been published to [crates.io](https://crates.io/crates/usn-parser), you can install it using Cargo:
```bash
cargo install usn-parser
```

Alternatively, you can download the latest release from the [Releases page](https://github.com/wangfu91/usn-parser-rs/releases/latest) and run the executable directly.

## 📖 Usage

 > Note: Administrator privileges are required to access USN journals and the MFT.

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

### 💡Examples

#### 👀 Monitor real-time USN journal changes.

```powershell
# Monitor drive C for real-time file changes, filtering for log files with the name prefix 'app':
usn-parser monitor C -f "app*.log" --file-only
```

#### 🔎 Search the MFT.
```powershell
# Search the MFT of drive C, printing out all files with the extension `.xlsx`:
usn-parser search C -f "*.xlsx" --file-only
```

#### 📖 Read history USN journal entries.
```powershell
# Print out the change history for file 'report.docx' from the USN journal of drive D:
usn-parser read D -f "report.docx"
```

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request or open an issue.

## 📜 License

This project is licensed under the terms of the [MIT LICENSE](LICENSE).