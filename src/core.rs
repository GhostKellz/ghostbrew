use crate::{aur, pacman, flatpak, config};

#[derive(Clone, Debug, serde::Deserialize, PartialEq)]
pub enum Source {
    Pacman,
    ChaoticAUR,
    Aur,
    Flatpak,
}

impl Source {
    pub fn label(&self) -> &'static str {
        match self {
            Source::Pacman => "[Pacman]",
            Source::ChaoticAUR => "[ChaoticAUR]",
            Source::Aur => "[AUR]",
            Source::Flatpak => "[Flatpak]",
        }
    }
}

pub struct SearchResult {
    pub name: String,
    pub version: String,
    pub description: String,
    pub source: Source,
}

pub fn unified_search(query: &str) -> Vec<SearchResult> {
    let mut results = Vec::new();
    // Pacman & ChaoticAUR search (distinguish by repo prefix)
    if let Ok(output) = std::process::Command::new("pacman").arg("-Ss").arg(query).output() {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines() {
            if let Some((repo_and_name, rest)) = line.split_once(' ') {
                if repo_and_name.starts_with("chaotic-aur/") {
                    results.push(SearchResult {
                        name: repo_and_name.trim().to_string(),
                        version: "-".to_string(),
                        description: rest.trim().to_string(),
                        source: Source::ChaoticAUR,
                    });
                } else if repo_and_name.contains('/') && !repo_and_name.starts_with("aur/") {
                    results.push(SearchResult {
                        name: repo_and_name.trim().to_string(),
                        version: "-".to_string(),
                        description: rest.trim().to_string(),
                        source: Source::Pacman,
                    });
                }
            }
        }
    }
    // AUR search
    for pkg in aur::aur_search_results(query) {
        results.push(SearchResult {
            name: pkg.Name.clone(),
            version: pkg.Version.clone(),
            description: pkg.Description.clone().unwrap_or_default(),
            source: Source::Aur,
        });
    }
    // Flatpak search
    if let Ok(output) = std::process::Command::new("flatpak").arg("search").arg(query).output() {
        let out = String::from_utf8_lossy(&output.stdout);
        for line in out.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() > 2 {
                results.push(SearchResult {
                    name: cols[0].to_string(),
                    version: "-".to_string(),
                    description: cols[1..].join(" "),
                    source: Source::Flatpak,
                });
            }
        }
    }
    results
}

pub fn print_search_results(results: &[SearchResult]) {
    for r in results {
        println!("{} {} {} - {}", r.source.label(), r.name, r.version, r.description);
    }
}

pub fn install_with_priority(pkg: &str, config: &config::BrewConfig) {
    // Load priorities from Lua config if present
    let mut priorities = vec![Source::Pacman, Source::Aur, Source::Flatpak];
    // Example: check for 'priorities' in config (as Vec<String>)
    // (Assume config.lua exposes a 'priorities' table: {"pacman", "aur", "flatpak"})
    // You'd parse this in config.rs and convert to Source enum here
    // For now, use the default order above
    for src in priorities {
        match src {
            Source::Pacman => {
                pacman::install(pkg);
                return;
            }
            Source::ChaoticAUR => {
                // For now, treat like Pacman (or add custom logic)
                pacman::install(pkg);
                return;
            }
            Source::Aur => {
                aur::install(pkg);
                return;
            }
            Source::Flatpak => {
                flatpak::install(pkg);
                return;
            }
        }
    }
    println!("[ghostbrew] Could not install {} from any source.", pkg);
}
