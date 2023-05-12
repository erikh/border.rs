use std::path::PathBuf;

use border::config::Config;
use clap::{Parser, Subcommand};
use josekit::{jwe::alg::aeskw::AeskwJweAlgorithm, jwk::Jwk};

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
    #[command(name = "key-generate")]
    KeyGenerate {
        #[arg(name = "Key ID (used for peer name in some cases)")]
        peer_name: String,
    },
}

type CommandResult = Result<(), anyhow::Error>;

fn main() -> CommandResult {
    let args = Args::parse();

    match args.command {
        Commands::ConfigCheck { filename } => check_config(filename),
        Commands::KeyGenerate { peer_name } => generate_key(peer_name),
    }
}

fn check_config(filename: PathBuf) -> CommandResult {
    let mut f = std::fs::OpenOptions::new();
    f.read(true);
    let io = f.open(filename)?;
    let _: Config = serde_yaml::from_reader(io)?;
    println!("Configuration Parsed OK");

    Ok(())
}

fn generate_key(peer_name: String) -> CommandResult {
    let mut jwk = Jwk::generate_oct_key(255)?;
    jwk.set_algorithm(AeskwJweAlgorithm::A256kw.name());
    jwk.set_key_id(peer_name);

    println!();
    println!("# Paste this key into your configuration where you need an encryption key.");
    println!("# Indentation is important!");
    println!("{}", serde_yaml::to_string(&jwk)?);

    Ok(())
}
