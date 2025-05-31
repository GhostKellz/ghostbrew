// Pacman repo logic
use std::process::Command;

/// Install a package from the official repositories using pacman
pub fn install(package: &str) {
    println!("[pacman] Installing package: {}", package);
    let status = Command::new("sudo")
        .arg("pacman")
        .arg("-S")
        .arg("--noconfirm")
        .arg(package)
        .status();
    if let Ok(s) = status {
        if s.success() {
            println!("[pacman] {} installed successfully!", package);
        } else {
            eprintln!("[pacman] pacman failed for {}", package);
        }
    } else {
        eprintln!("[pacman] failed to run pacman for {}", package);
    }
}
