use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "ghostbrew")]
#[command(version = "0.2.0")]
#[command(about = "A Rust-powered AUR helper", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Search the AUR for a package
    Search {
        query: String,
    },
    /// Install a package
    Install {
        package: String,
    },
    /// Upgrade installed AUR packages
    Upgrade,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Search { query } => {
            println!("🔍 Searching for package: {query}");
            // stub
        }
        Commands::Install { package } => {
            println!("📦 Installing package: {package}");
            // stub
        }
        Commands::Upgrade => {
            println!("♻️ Upgrading AUR packages...");
            // stub
        }
    }
}

