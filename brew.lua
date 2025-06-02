-- Example ghostbrew Lua config
ignored_packages = { "linux", "nvidia", "linux-firmware", "mesa" }
parallel = 8
priorities = { "chaotic-aur", "aur", "pacman", "flatpak" }

-- Custom pre/post hooks for install, upgrade, remove
function pre_install(pkg)
  print("[hook] About to install " .. pkg)
  if pkg == "linux" then
    print("[hook] Warning: Installing a kernel!")
  end
end

function post_install(pkg)
  print("[hook] Finished installing " .. pkg)
end

function pre_upgrade()
  print("[hook] Starting system upgrade...")
end

function post_upgrade()
  print("[hook] System upgrade complete!")
end

function pre_remove(pkg)
  print("[hook] About to remove " .. pkg)
end

function post_remove(pkg)
  print("[hook] Removed " .. pkg)
end

-- Custom audit rule example
audit_keywords = { "curl", "wget", "sudo", "rm -rf", "chmod", "chown", "dd", "mkfs", "mount", "scp", "nc", "ncat", "bash -c", "eval" }
function custom_audit(pkgbuild)
  for _, keyword in ipairs(audit_keywords) do
    if string.find(pkgbuild, keyword) then
      print("[AUDIT][LUA] Found risky command: " .. keyword)
    end
  end
end
