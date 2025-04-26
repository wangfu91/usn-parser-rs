use clap::{Parser, Subcommand};
use usn_journal_rs::{
    mft::Mft,
    path_resolver::PathResolver,
    usn_journal::{self, UsnJournal, UsnJournalEnumOptions},
    utils, USN_REASON_MASK_ALL,
};

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

    let journal_data = usn_journal::query(volume_handle, true)?;

    println!("Journal data: {:#?}", journal_data);

    let mut path_resolver = PathResolver::new(volume_handle, volume.chars().next().unwrap());

    match cli.command {
        Commands::Monitor {} => {
            let options = UsnJournalEnumOptions {
                start_usn: journal_data.NextUsn,
                reason_mask: USN_REASON_MASK_ALL,
                only_on_close: true,
                timeout: 0,
                wait_for_more: true,
                ..Default::default()
            };
            let usn_journal =
                UsnJournal::new_with_options(volume_handle, journal_data.UsnJournalID, options);
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
            let mft = Mft::new(volume_handle);
            for entry in mft {
                let full_path =
                    path_resolver.resolve_path(entry.fid, entry.parent_fid, &entry.file_name);
                println!("fid={:?}, path={:?}", entry.fid, full_path);
            }
        }
    }

    Ok(())
}
