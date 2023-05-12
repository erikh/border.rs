use std::path::PathBuf;

use border::config::Config;
use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    author = "Erik Hollensbe <erik+github@hollensbe.org",
    about = "Generate a tree of documents for testing DID parser compliance"
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(name = "config-check")]
    ConfigCheck { filename: PathBuf },
}

fn main() -> Result<(), anyhow::Error> {
    let args = Args::parse();

    match args.command {
        Commands::ConfigCheck { filename } => {
            let mut f = std::fs::OpenOptions::new();
            f.read(true);
            let io = f.open(filename)?;
            let _: Config = serde_yaml::from_reader(io)?;
            println!("Configuration Parsed OK");
        }
    }

    Ok(())
}
