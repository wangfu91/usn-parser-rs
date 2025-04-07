mod mft;
mod path_resolver;
mod usn_entry;
mod usn_journal;
mod utils;

use clap::{Parser, Subcommand};
use mft::Mft;
use path_resolver::PathResolver;
use usn_journal::UsnJournal;

#[derive(Parser, Debug)]
#[command(name = "usn-parser")]
#[command(
    about = "NTFS USN Journal parser",
    long_about = "A command utility for NTFS to search the MFT & monitoring the changes of USN Journal."
)]
struct Cli {
    volume: String,

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

    let volume = cli.volume;

    let volume_handle = utils::get_volume_handle(&volume)?;

    println!("volume handle = {:?}", volume_handle);

    let journal_data = usn_journal::query_usn_info(volume_handle)?;

    println!("Journal data: {:#?}", journal_data);

    let mut path_resolver = PathResolver::new(volume_handle, volume.chars().next().unwrap());

    match cli.command {
        Commands::Monitor {} => {
            let usn_journal = UsnJournal::new(
                volume_handle,
                journal_data.UsnJournalID,
                journal_data.NextUsn,
            );
            for entry in usn_journal {
                let full_path =
                    path_resolver.resolve_path(entry.fid, entry.parent_fid, &entry.file_name);
                println!(
                    "usn={:?}, file_id={:?}, path={:?}",
                    entry.usn, entry.fid, full_path
                );
            }
        }

        Commands::Mft {} => {
            let mft = Mft::new(volume_handle, journal_data.NextUsn);
            for entry in mft {
                let full_path =
                    path_resolver.resolve_path(entry.fid, entry.parent_fid, &entry.file_name);
                println!("fid={:?}, path={:?}", entry.fid, full_path);
            }
        }
    }

    Ok(())
}
