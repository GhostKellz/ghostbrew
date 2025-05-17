package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"sync"
)

type InstallOptions struct {
	Parallel int // Number of parallel jobs
}

// AURInfoCache caches AUR info responses
var AURInfoCache = struct {
	m map[string]map[string]interface{}
	sync.Mutex
}{m: make(map[string]map[string]interface{})}

// fetchAURInfo fetches AUR info for a package (including dependencies), with cache
func fetchAURInfo(pkg string) (map[string]interface{}, error) {
	AURInfoCache.Lock()
	if info, ok := AURInfoCache.m[pkg]; ok {
		AURInfoCache.Unlock()
		return info, nil
	}
	AURInfoCache.Unlock()
	resp, err := http.Get("https://aur.archlinux.org/rpc/?v=5&type=info&arg=" + pkg)
	if err != nil {
		return nil, err
	}
	defer resp.Body.Close()
	var result struct {
		Results []map[string]interface{}
	}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil || len(result.Results) == 0 {
		return nil, fmt.Errorf("No info found for %s", pkg)
	}
	info := result.Results[0]
	AURInfoCache.Lock()
	AURInfoCache.m[pkg] = info
	AURInfoCache.Unlock()
	return info, nil
}

// resolveDependencies concurrently resolves dependencies for a list of packages
func resolveDependencies(pkgs []string, seen map[string]bool) ([]string, error) {
	var order []string
	var mu sync.Mutex
	var wg sync.WaitGroup
	for _, pkg := range pkgs {
		if seen[pkg] {
			continue
		}
		seen[pkg] = true
		wg.Add(1)
		go func(pkg string) {
			defer wg.Done()
			info, err := fetchAURInfo(pkg)
			if err != nil {
				fmt.Println("Dependency fetch failed:", err)
				return
			}
			var depOrder []string
			if deps, ok := info["Depends"].([]interface{}); ok {
				depNames := make([]string, 0)
				for _, d := range deps {
					if depStr, ok := d.(string); ok {
						depNames = append(depNames, depStr)
					}
				}
				depOrder, _ = resolveDependencies(depNames, seen)
			}
			mu.Lock()
			order = append(order, depOrder...)
			order = append(order, pkg)
			mu.Unlock()
		}(pkg)
	}
	wg.Wait()
	return order, nil
}

// checkGPGKey checks and imports GPG keys if missing (stub)
func checkGPGKey(pkg string) {
	// TODO: Implement real GPG key check and import logic
	fmt.Printf("[GPG] Checking keys for %s...\n", pkg)
}

// inspectPKGBUILD fetches and inspects PKGBUILD for risky commands (stub)
func inspectPKGBUILD(pkg string) {
	// TODO: Fetch PKGBUILD and highlight risky commands
	fmt.Printf("[AUDIT] Inspecting PKGBUILD for %s...\n", pkg)
}

func InstallPackages(pkgs []string, opts InstallOptions) {
	seen := make(map[string]bool)
	order, err := resolveDependencies(pkgs, seen)
	if err != nil {
		fmt.Println("Dependency resolution failed:", err)
		return
	}
	var wg sync.WaitGroup
	sem := make(chan struct{}, opts.Parallel)
	for _, pkg := range order {
		wg.Add(1)
		go func(pkg string) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			checkGPGKey(pkg)
			inspectPKGBUILD(pkg)
			fmt.Printf("Building and installing %s...\n", pkg)
			cmd := exec.Command("echo", "Simulating build/install for "+pkg)
			cmd.Stdout = os.Stdout
			cmd.Stderr = os.Stderr
			_ = cmd.Run()
		}(pkg)
	}
	wg.Wait()
}
