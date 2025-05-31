// Plugin/hook system (scaffold)
/// Run user-defined scripts at lifecycle events (pre/post install, etc.)
/// This will look for a Lua function in the user's config and call it if present.
pub fn run_hook(hook: &str, pkg: &str) {
    use mlua::Lua;
    use std::fs;
    use std::path::PathBuf;
    let config_path = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".config/ghostbrew/brew.lua");
    if let Ok(script) = fs::read_to_string(&config_path) {
        let lua = Lua::new();
        if let Ok(_) = lua.load(&script).exec() {
            let globals = lua.globals();
            if let Ok(func) = globals.get::<_, mlua::Function>(hook) {
                let _ = func.call::<_, ()>(pkg);
                println!("[ghostbrew] Ran Lua hook '{}' for package '{}'.", hook, pkg);
            } else {
                println!("[ghostbrew] No Lua hook '{}' defined in config.", hook);
            }
        }
    }
}
