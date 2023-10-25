use clap::{Parser, Subcommand};
use usn_change_journal::journal::UsnJournal;

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

    let volume_root = format!(r"\\.\{}", volume);
    // println!("volume_root={}", volume_root);

    let usn_journal = UsnJournal {};

    let volume_handle = usn_journal.get_volume_handle(&volume_root)?;

    // println!("volume handle = {:?}", volume_handle);

    let journal_data = usn_journal.query_usn_state(&volume_handle)?;

    // println!("Journal data: {:#?}", journal_data);

    match cli.command {
        Commands::Monitor {} => {
            usn_journal.monitor_usn_journal(&volume_handle, &journal_data)?;
        }

        Commands::Mft {} => {
            usn_journal.read_mft(&volume_handle, &journal_data)?;
        }
    }

    Ok(())
}
