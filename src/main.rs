use clap::{Parser, Subcommand};
use usn_journal_rs::{
    USN_REASON_MASK_ALL,
    journal::{self, UsnJournal},
    mft::{self, Mft},
    path::{JournalPathResolver, MftPathResolver},
    volume::Volume,
};
use wax::{Glob, Pattern};

#[derive(Parser, Debug)]
#[command(name = "usn-parser")]
#[command(
    version, // Reads version from Cargo.toml
    about = "NTFS USN Journal parser",
    long_about = "A command utility for NTFS to search the MFT & monitoring the changes of USN Journal."
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    #[command(about = "Monitor real-time USN journal changes")]
    Monitor(FilterOptions),

    #[command(about = "Search the Master File Table")]
    Search(FilterOptions),

    #[command(about = "Read history USN journal entries")]
    Read(FilterOptions),
}

impl Commands {
    fn volume(&self) -> char {
        match self {
            Commands::Monitor(args) | Commands::Search(args) | Commands::Read(args) => args.volume,
        }
    }
}

#[derive(Parser, Debug)]
struct FilterOptions {
    #[arg(help = "Volume name, e.g. C")]
    volume: char,

    #[arg(
        short('f'),
        long("filter"),
        help = "Filter the result with keyword, wildcards are permitted"
    )]
    keyword: Option<String>,

    #[arg(long = "file-only", help = "Only show the file entries")]
    file_only: bool,

    #[arg(long = "dir-only", help = "Only show the directory entries")]
    directory_only: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Extract shared arguments and volume based on the subcommand
    let drive_letter = cli.command.volume();
    let volume = Volume::from_drive_letter(drive_letter)?;

    match cli.command {
        Commands::Monitor(args) => {
            let usn_journal = UsnJournal::new(volume);
            let journal_data = usn_journal.query(true)?;
            let options = journal::EnumOptions {
                start_usn: journal_data.next_usn,
                reason_mask: USN_REASON_MASK_ALL,
                only_on_close: true,
                timeout: 0,
                wait_for_more: true,
                ..Default::default()
            };
            let mut path_resolver = JournalPathResolver::new(&usn_journal);
            let glob = if let Some(ref filter) = args.keyword {
                Some(Glob::new(filter)?)
            } else {
                None
            };
            for entry in usn_journal.iter_with_options(options)? {
                if args.file_only && entry.is_dir() {
                    continue;
                }
                if args.directory_only && !entry.is_dir() {
                    continue;
                }
                if let Some(ref glob) = glob {
                    if !glob.is_match(entry.file_name.as_os_str()) {
                        continue;
                    }
                }

                let full_path = path_resolver.resolve_path(&entry);
                println!(
                    "usn={:?}, file_id={:?}, path={:?}",
                    entry.usn, entry.fid, full_path
                );
            }
        }

        Commands::Search(args) => {
            let options = mft::EnumOptions {
                low_usn: 0,
                high_usn: i64::MAX,
                ..Default::default()
            };
            let mft = Mft::new(volume);
            let mut path_resolver = MftPathResolver::new(&mft);
            let glob = if let Some(ref filter) = args.keyword {
                Some(Glob::new(filter)?)
            } else {
                None
            };
            for entry in mft.iter_with_options(options) {
                if args.file_only && entry.is_dir() {
                    continue;
                }
                if args.directory_only && !entry.is_dir() {
                    continue;
                }
                if let Some(ref glob) = glob {
                    if !glob.is_match(entry.file_name.as_os_str()) {
                        continue;
                    }
                }
                let full_path = path_resolver.resolve_path(&entry);
                println!("fid={:?}, path={:?}", entry.fid, full_path);
            }
        }
        Commands::Read(args) => {
            let usn_journal = UsnJournal::new(volume);
            let options = journal::EnumOptions {
                reason_mask: USN_REASON_MASK_ALL,
                ..Default::default()
            };
            let mut path_resolver = JournalPathResolver::new(&usn_journal);
            let glob = if let Some(ref filter) = args.keyword {
                Some(Glob::new(filter)?)
            } else {
                None
            };
            for entry in usn_journal.iter_with_options(options)? {
                if args.file_only && entry.is_dir() {
                    continue;
                }
                if args.directory_only && !entry.is_dir() {
                    continue;
                }
                if let Some(ref glob) = glob {
                    if !glob.is_match(entry.file_name.as_os_str()) {
                        continue;
                    }
                }
                let full_path = path_resolver.resolve_path(&entry);
                println!(
                    "usn={:?}, file_id={:?}, path={:?}, reason={:?}",
                    entry.usn, entry.fid, full_path, entry.reason
                );
            }
        }
    }

    Ok(())
}
