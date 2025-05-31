mod tui;
mod aur;
mod pacman;
mod config;
mod gpg;
mod hooks;
mod utils;
mod core;
mod flatpak;

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
    /// Launch the interactive TUI
    Tui,
    /// Search the AUR for a package
    Search {
        query: String,
    },
    /// Install a package
    Install {
        package: String,
    },
    /// Upgrade installed packages
    Upgrade,
    /// Add a private tap/repo
    Tap {
        repo: String,
    },
    /// Run a shell completion script
    Completion {
        shell: String,
    },
    /// Rollback a package to previous version
    Rollback {
        package: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Tui => tui::run(),
        Commands::Search { query } => aur::search(query),
        Commands::Install { package } => {
            // PKGBUILD diff/audit before install
            let pkgb = aur::get_pkgbuild_preview(&package);
            utils::pkgb_diff_audit(&package, &pkgb);
            aur::install(&package);
        },
        Commands::Upgrade => {
            // For each upgradable package, show PKGBUILD diff/audit and changelog before upgrade
            // (You can expand aur::upgrade to handle this per-package)
            aur::upgrade();
        },
        Commands::Tap { repo } => aur::add_tap(repo),
        Commands::Completion { shell } => utils::completion(shell),
        Commands::Rollback { package } => {
            utils::rollback(package);
        },
    }
}

