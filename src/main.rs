use clap::{Parser, Subcommand};
use usn_journal_rs::{
    USN_REASON_MASK_ALL,
    journal::{self, UsnJournal},
    mft::{self, Mft},
    path::{JournalPathResolver, MftPathResolver},
    volume::Volume,
};

#[derive(Parser, Debug)]
#[command(name = "usn-parser")]
#[command(
    about = "NTFS USN Journal parser",
    long_about = "A command utility for NTFS to search the MFT & monitoring the changes of USN Journal."
)]
struct Cli {
    volume: char,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    Monitor {},

    Mft {},
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let drive_letter = cli.volume;
    let volume = Volume::from_drive_letter(drive_letter)?;

    let journal_data = journal::query(&volume, true)?;

    println!("Journal data: {:#?}", journal_data);

    match cli.command {
        Commands::Monitor {} => {
            let options = journal::EnumOptions {
                start_usn: journal_data.NextUsn,
                reason_mask: USN_REASON_MASK_ALL,
                only_on_close: true,
                timeout: 0,
                wait_for_more: true,
                ..Default::default()
            };
            let usn_journal = UsnJournal::new(volume)?;
            let mut path_resolver = JournalPathResolver::new(&usn_journal);
            for entry in usn_journal.iter_with_options(options) {
                let full_path = path_resolver.resolve_path(&entry);
                println!(
                    "usn={:?}, file_id={:?}, path={:?}",
                    entry.usn, entry.fid, full_path
                );
            }
        }

        Commands::Mft {} => {
            let options = mft::EnumOptions {
                low_usn: 0,
                high_usn: i64::MAX,
                ..Default::default()
            };
            let mft = Mft::new(volume)?;
            let mut path_resolver = MftPathResolver::new(&mft);
            for entry in mft.iter_with_options(options) {
                let full_path = path_resolver.resolve_path(&entry);
                println!("fid={:?}, path={:?}", entry.fid, full_path);
            }
        }
    }

    Ok(())
}
