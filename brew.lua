-- Example ghostbrew Lua config
ignored_packages = { "linux", "nvidia" }
parallel = 20
priorities = { "chaotic-aur", "aur", "pacman", "flatpak" }

function pre_install(pkg)
  print("[hook] About to install " .. pkg)
end

function post_install(pkg)
  print("[hook] Finished installing " .. pkg)
end
