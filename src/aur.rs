#[derive(Clone, Debug, serde::Deserialize)]
pub struct AurResult {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Version")]
    pub version: String,
    #[serde(rename = "Description")]
    pub description: Option<String>,
    #[serde(rename = "Maintainer")]
    pub maintainer: Option<String>,
    #[serde(rename = "URL")]
    pub url: Option<String>,
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct AurResponse {
    pub results: Vec<AurResult>,
}

use crate::utils;

pub fn search(query: &str) {
    let results = aur_search_results(query);
    for result in results {
        let desc = result.description.as_deref().unwrap_or("");
        let maint = result.maintainer.as_deref().unwrap_or("");
        println!(
            "[AUR] {} {} {} - {}",
            result.name,
            result.version,
            maint,
            desc
        );
    }
}

pub fn aur_search_results(query: &str) -> Vec<AurResult> {
    let url = format!("https://aur.archlinux.org/rpc/?v=5&type=search&arg={}", query);
    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(json) = resp.json::<AurResponse>() {
            return json.results;
        }
    }
    vec![]
}

fn prompt_confirm(msg: &str) -> bool {
    use std::io::{self, Write};
    print!("{} [y/N]: ", msg);
    let _ = io::stdout().flush();
    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        matches!(input.trim().to_lowercase().as_str(), "y" | "yes")
    } else {
        false
    }
}

pub fn install(package: &str) {
    if !prompt_confirm(&format!("Install AUR package {}?", package)) {
        println!("[ghostbrew] Skipped install for {}", package);
        return;
    }
    let aur_url = format!("https://aur.archlinux.org/{}.git", package);
    let tmp_dir = std::env::temp_dir().join(format!("ghostbrew-aur-{}", package));
    let _ = std::fs::remove_dir_all(&tmp_dir);
    let status = std::process::Command::new("git")
        .arg("clone").arg(&aur_url).arg(&tmp_dir)
        .status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        eprintln!("[ghostbrew] Failed to clone AUR repo for {}", package);
        return;
    }
    let status = std::process::Command::new("makepkg")
        .current_dir(&tmp_dir)
        .arg("-si").arg("--noconfirm")
        .status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        eprintln!("[ghostbrew] makepkg failed for {}", package);
    }
    let _ = std::fs::remove_dir_all(&tmp_dir);
}

pub fn get_pkgbuild_preview(pkg: &str) -> String {
    let url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", pkg);
    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(text) = resp.text() {
            return text;
        }
    }
    String::from("[ghostbrew] PKGBUILD not found.")
}

pub fn get_deps(pkg: &str) -> Vec<String> {
    let pkgb = get_pkgbuild_preview(pkg);
    let mut deps = Vec::new();
    let mut in_dep = false;
    let mut dep_buf = String::new();
    for line in pkgb.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("depends=") {
            in_dep = true;
            dep_buf.push_str(trimmed.split_once('=').map(|x| x.1).unwrap_or("").trim());
            if trimmed.ends_with(')') { in_dep = false; }
        } else if in_dep {
            dep_buf.push_str(trimmed);
            if trimmed.ends_with(')') { in_dep = false; }
        }
        if !in_dep && !dep_buf.is_empty() {
            let dep_line = dep_buf.trim_matches(&['(', ')', '"', '\'', ' '] as &[_]);
            deps.extend(dep_line.split_whitespace().map(|s| s.trim_matches(&['"', '\'', ' '] as &[_])).filter(|s| !s.is_empty()).map(|s| s.to_string()));
            dep_buf.clear();
        }
    }
    deps
}

use std::sync::mpsc;

pub fn upgrade() {
    println!("[ghostbrew] Upgrading system packages...");
    let _ = std::process::Command::new("sudo").arg("pacman").arg("-Syu").status();
    let output = std::process::Command::new("pacman").arg("-Qm").output();
    if let Ok(out) = output {
        let pkgs = String::from_utf8_lossy(&out.stdout);
        let config = crate::config::BrewConfig::load();
        let (tx, rx) = mpsc::channel();
        let mut _count = 0;
        for line in pkgs.lines() {
            let pkg = line.split_whitespace().next().unwrap_or("").to_string();
            if !pkg.is_empty() && !config.is_ignored(&pkg) {
                let tx = tx.clone();
                _count += 1;
                utils::backup_package(&pkg);
                install(&pkg);
                let _ = tx.send(pkg);
            }
        }
        drop(tx);
        for pkg in rx {
            println!("[ghostbrew] Finished upgrade for {}", pkg);
        }
    }
}

pub fn add_tap(repo: &str) {
    let taps_dir = dirs::home_dir().unwrap_or_default().join(".local/share/ghostbrew/taps");
    let _ = std::fs::create_dir_all(&taps_dir);
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let dest = taps_dir.join(repo_name);
    let _ = std::fs::remove_dir_all(&dest);
    let status = std::process::Command::new("git")
        .arg("clone").arg(repo).arg(&dest)
        .status();
    if !status.map(|s| s.success()).unwrap_or(false) {
        eprintln!("[ghostbrew] Failed to add tap: {}", repo);
    } else {
        println!("[ghostbrew] Tap added: {}", repo);
    }
}

pub fn remove_tap(repo: &str) {
    let taps_dir = dirs::home_dir().unwrap_or_default().join(".local/share/ghostbrew/taps");
    let repo_name = repo.split('/').next_back().unwrap_or(repo);
    let dest = taps_dir.join(repo_name);
    if dest.exists() {
        let _ = std::fs::remove_dir_all(&dest);
        println!("[ghostbrew] Tap removed: {}", repo);
    } else {
        eprintln!("[ghostbrew] Tap not found: {}", repo);
    }
}

pub fn list_taps() {
    let taps_dir = dirs::home_dir().unwrap_or_default().join(".local/share/ghostbrew/taps");
    if let Ok(entries) = std::fs::read_dir(&taps_dir) {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                println!("[ghostbrew] Tap: {}", name);
            }
        }
    } else {
        println!("[ghostbrew] No taps found.");
    }
}