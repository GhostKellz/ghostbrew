mod aur;
mod config;
mod core;
mod flatpak;
mod gpg;
mod hooks;
mod pacman;
mod tui;
mod utils;

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
    Search { query: String },
    /// Install a package
    Install {
        package: String,
        #[arg(long)]
        gpg_key: Option<String>,
    },
    /// Upgrade installed packages
    Upgrade,
    /// Add a private tap/repo
    Tap { repo: String },
    /// Run a shell completion script
    Completion { shell: String },
    /// Rollback a package to previous version
    Rollback { package: String },
    /// Rollback PKGBUILD to previous version
    RollbackPkgb { package: String },
    /// Set the GPG keyserver
    SetKeyserver { keyserver: String },
    /// Flatpak search
    FlatpakSearch { query: String },
    /// Flatpak upgrade
    FlatpakUpgrade,
    /// Flatpak sandbox info
    FlatpakSandboxInfo { package: String },
    /// Search the AUR directly
    AurSearch { query: String },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Tui => tui::run(),
        Commands::Search { query } => {
            let results = core::unified_search(query);
            core::print_search_results(&results);
            // Show maintainer for AUR results
            for r in &results {
                if r.source == core::Source::Aur {
                    // Use aur::aur_search_results to get full AurResult with maintainer
                    for aur in aur::aur_search_results(&r.name) {
                        if let Some(maint) = &aur.maintainer {
                            println!("[AUR] Maintainer for {}: {}", aur.name, maint);
                        }
                    }
                }
            }
        }
        Commands::Install { package, gpg_key } => {
            if let Some(key) = gpg_key {
                gpg::check_key(key);
            }
            hooks::run_hook("pre_install", package);
            let pkgb = aur::get_pkgbuild_preview(package);
            utils::pkgb_diff_audit(package, &pkgb);
            aur::install(package);
            hooks::run_hook("post_install", package);
        }
        Commands::Upgrade => {
            hooks::run_hook("pre_upgrade", "");
            aur::upgrade();
            flatpak::upgrade();
            hooks::run_hook("post_upgrade", "");
        }
        Commands::Tap { repo } => aur::add_tap(repo),
        Commands::Completion { shell } => utils::completion(shell),
        Commands::Rollback { package } => {
            utils::rollback(package);
        }
        Commands::RollbackPkgb { package } => {
            utils::cli_rollback_pkgbuild(package);
        }
        Commands::SetKeyserver { keyserver } => {
            utils::cli_set_keyserver(keyserver);
        }
        Commands::FlatpakSearch { query } => {
            flatpak::search(query);
        }
        Commands::FlatpakUpgrade => {
            flatpak::upgrade();
        }
        Commands::FlatpakSandboxInfo { package } => {
            flatpak::print_flatpak_sandbox_info(package);
        }
        Commands::AurSearch { query } => {
            aur::search(query);
        }
    }
}
