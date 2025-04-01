mod mft;
mod usn_parser;
mod utils;

use clap::{Parser, Subcommand};

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

    let journal_data = usn_parser::query_usn_info(volume_handle)?;

    println!("Journal data: {:#?}", journal_data);

    match cli.command {
        Commands::Monitor {} => {
            usn_parser::monitor_usn_journal(volume_handle, &journal_data)?;
        }

        Commands::Mft {} => {
            mft::read_mft(volume_handle, &journal_data)?;
        }
    }

    Ok(())
}
