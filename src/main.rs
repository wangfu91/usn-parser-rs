// Unwraps
#![warn(clippy::unwrap_used)] // Discourage using .unwrap() which can cause panics
#![warn(clippy::expect_used)] // Discourage using .expect() which can cause panics
#![warn(clippy::panicking_unwrap)] // Prevent unwrap on values known to cause panics
#![warn(clippy::option_env_unwrap)] // Prevent unwrapping environment variables which might be absent

// Array indexing
#![warn(clippy::indexing_slicing)] // Avoid direct array indexing and use safer methods like .get()

// Path handling
#![warn(clippy::join_absolute_paths)] // Prevent issues when joining paths with absolute paths

// Serialization issues
#![warn(clippy::serde_api_misuse)] // Prevent incorrect usage of Serde's serialization/deserialization API

// Unbounded input
#![warn(clippy::uninit_vec)] // Prevent creating uninitialized vectors which is unsafe

// Unsafe code detection
#![warn(clippy::transmute_int_to_char)] // Prevent unsafe transmutation from integers to characters
#![warn(clippy::transmute_int_to_float)] // Prevent unsafe transmutation from integers to floats
#![warn(clippy::transmute_ptr_to_ref)] // Prevent unsafe transmutation from pointers to references
#![warn(clippy::transmute_undefined_repr)] // Detect transmutes with potentially undefined representations

mod mft;
mod path_resolver;
mod usn_entry;
mod usn_journal;
mod utils;

use clap::{Parser, Subcommand};
use mft::Mft;
use path_resolver::PathResolver;
use usn_journal::{UsnJournal, UsnJournalEnumOptions};

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
            let options = UsnJournalEnumOptions {
                start_usn: journal_data.NextUsn,
                reason_mask: 0xFFFFFFFF,
                only_on_close: true,
                timeout: 0,
                wait_for_more: true,
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
