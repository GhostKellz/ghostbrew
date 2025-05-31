use mlua::Lua;
use std::fs;
use std::path::PathBuf;

pub struct BrewConfig {
    /// Packages to ignore during upgrades (from Lua config)
    pub ignored_packages: Vec<String>,
    /// Number of parallel jobs for install/upgrade (from Lua config)
    pub parallel: usize,
    // Add more config fields as needed
}

impl BrewConfig {
    pub fn load() -> Self {
        let config_path = dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("/tmp"))
            .join(".config/ghostbrew/brew.lua");
        let lua = Lua::new();
        let mut ignored_packages = Vec::new();
        let mut parallel = 2;
        if let Ok(script) = fs::read_to_string(&config_path) {
            if let Ok(table) = lua.load(&script).eval::<mlua::Table>() {
                if let Ok(pkgs) = table.get::<_, mlua::Table>("ignored_packages") {
                    for pkg in pkgs.sequence_values::<String>().flatten() {
                        ignored_packages.push(pkg);
                    }
                }
                if let Ok(p) = table.get::<_, usize>("parallel") {
                    parallel = p;
                }
            }
        }
        BrewConfig { ignored_packages, parallel }
    }

    /// Check if a package is ignored (used in upgrade/install logic)
    pub fn is_ignored(&self, pkg: &str) -> bool {
        self.ignored_packages.iter().any(|p| p == pkg)
    }
}
