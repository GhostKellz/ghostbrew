use std::process::Command;

// Flatpak integration (scaffold)
pub fn search(query: &str) {
    println!("[flatpak] Searching for: {}", query);
    let output = Command::new("flatpak").arg("search").arg(query).output();
    match output {
        Ok(out) => {
            let results = String::from_utf8_lossy(&out.stdout);
            println!("{}", results);
        }
        Err(e) => println!("[flatpak] search failed: {}", e),
    }
}

pub fn install(package: &str) {
    println!("[flatpak] Installing: {}", package);
    let status = Command::new("flatpak").arg("install").arg("-y").arg(package).status();
    match status {
        Ok(s) if s.success() => println!("[flatpak] {} installed successfully!", package),
        Ok(_) | Err(_) => println!("[flatpak] install failed for {}", package),
    }
}

pub fn upgrade() {
    println!("[flatpak] Upgrading all flatpak packages...");
    let status = Command::new("flatpak").arg("update").arg("-y").status();
    match status {
        Ok(s) if s.success() => println!("[flatpak] All packages upgraded!"),
        Ok(_) | Err(_) => println!("[flatpak] upgrade failed."),
    }
}
