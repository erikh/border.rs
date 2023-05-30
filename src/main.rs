use anyhow::anyhow;
use border::{config::Config, serve::Server};
use clap::{Parser, Subcommand};
use josekit::{jwe::alg::aeskw::AeskwJweAlgorithm, jwk::Jwk};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::Mutex;

#[derive(Parser, Debug)]
#[command(
    author = "Erik Hollensbe <erik+github@hollensbe.org",
    about = "border: Clustered Load Balancer & DNS Service - to keep things alive above all"
)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    #[command(
        name = "config-check",
        about = "Validate your configuration and ensure it parses"
    )]
    ConfigCheck { filename: PathBuf },
    #[command(
        name = "key-generate",
        about = "Generate a key used for client authentication, or peer authentication"
    )]
    KeyGenerate {
        #[arg(name = "Key ID (used for peer name in some cases)")]
        peer_name: String,
    },
    #[command(name = "serve", about = "Start border")]
    Serve {
        #[arg(name = "Configuration file")]
        filename: PathBuf,
        #[arg(name = "Peer name (must match a registered peer's `kid` in configuration file)")]
        peer: String,
    },
}

type CommandResult = Result<(), anyhow::Error>;

#[tokio::main]
async fn main() -> CommandResult {
    let args = Args::parse();

    match args.command {
        Commands::ConfigCheck { filename } => check_config(filename),
        Commands::KeyGenerate { peer_name } => generate_key(peer_name),
        Commands::Serve { filename, peer } => serve(filename, peer).await,
    }
}

async fn serve(filename: PathBuf, peer: String) -> CommandResult {
    let mut f = std::fs::OpenOptions::new();
    f.read(true);
    let io = f.open(filename)?;
    let mut config: Config = serde_yaml::from_reader(io)?;

    let mut found = false;

    for p in &config.peers {
        if p.name() == peer {
            found = true;
            break;
        }
    }

    if !found {
        return Err(anyhow!(
            "Peer `{}` is not listed in configuration file",
            peer
        ));
    }

    config.me = peer;

    let server = Server::new(Arc::new(Mutex::new(config)));
    server.serve().await
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
