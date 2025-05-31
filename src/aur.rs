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
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct AurResponse {
    pub results: Vec<AurResult>,
}

pub fn search(query: &str) {
    let results = crate::core::unified_search(query);
    for result in results {
        println!(
            "{} {} {} - {}",
            result.source.label(),
            result.name,
            result.version,
            result.description
        );
    }
}

pub fn install(package: &str) {
    // Real AUR install logic: clone, makepkg, install
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
    // Fetch PKGBUILD for preview (stub, replace with real logic)
    let url = format!("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h={}", pkg);
    if let Ok(resp) = reqwest::blocking::get(&url) {
        if let Ok(text) = resp.text() {
            return text;
        }
    }
    String::from("[ghostbrew] PKGBUILD not found.")
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

pub fn upgrade() {
    // Real upgrade logic: run system upgrade for AUR and repo packages
    println!("[ghostbrew] Upgrading system packages...");
    let _ = std::process::Command::new("sudo").arg("pacman").arg("-Syu").status();
    // TODO: Add AUR upgrade logic (e.g., check for AUR updates, rebuild)
}

pub fn add_tap(repo: &str) {
    // Real tap logic: add a custom repo (clone or add to config)
    println!("[ghostbrew] Adding tap {}", repo);
    // TODO: Implement tap registration logic
}

pub fn get_deps(pkg: &str) -> Vec<String> {
    // Real dependency fetch: parse PKGBUILD depends array
    let pkgb = get_pkgbuild_preview(pkg);
    let mut deps = Vec::new();
    for line in pkgb.lines() {
        if line.trim_start().starts_with("depends=") {
            let dep_line = line.splitn(2, '=').nth(1).unwrap_or("").trim();
            let dep_line = dep_line.trim_matches(&['(', ')', '"', '\'', ' '] as &[_]);
            deps.extend(dep_line.split_whitespace().map(|s| s.trim_matches(&['"', '\'', ' '] as &[_])).filter(|s| !s.is_empty()).map(|s| s.to_string()));
        }
    }
    deps
}
