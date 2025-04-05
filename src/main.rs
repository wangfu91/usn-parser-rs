mod mft;
mod path_resolver;
mod usn_entry;
mod usn_info;
//mod usn_parser;
mod utils;

use clap::{Parser, Subcommand};
use mft::Mft;
use path_resolver::PathResolver;

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

    let journal_data = usn_info::query_usn_info(volume_handle)?;

    println!("Journal data: {:#?}", journal_data);

    let mut path_resolver = PathResolver::new(volume_handle, volume.chars().next().unwrap());

    match cli.command {
        Commands::Monitor {} => {
            //usn_parser::monitor_usn_journal(volume_handle, &journal_data)?;
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
