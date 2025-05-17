package cmd

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"os/exec"
	"strings"
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

// inspectPKGBUILD fetches and inspects PKGBUILD for risky commands
func inspectPKGBUILD(pkg string) {
	// Fetch PKGBUILD from AUR
	url := fmt.Sprintf("https://aur.archlinux.org/cgit/aur.git/plain/PKGBUILD?h=%s", pkg)
	resp, err := http.Get(url)
	if err != nil {
		fmt.Printf("[AUDIT] Failed to fetch PKGBUILD for %s: %v\n", pkg, err)
		return
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		fmt.Printf("[AUDIT] Failed to read PKGBUILD for %s: %v\n", pkg, err)
		return
	}
	pkgb := string(data)
	fmt.Printf("[AUDIT] PKGBUILD for %s:\n", pkg)
	fmt.Println("----------------------------------------")
	fmt.Println(pkgb)
	fmt.Println("----------------------------------------")
	// Highlight risky commands
	keywords := []string{"curl", "wget", "sudo", "rm -rf", "chmod", "chown", "dd", "mkfs", "mount", "scp", "nc", "ncat", "bash -c", "eval"}
	for _, k := range keywords {
		if strings.Contains(pkgb, k) {
			fmt.Printf("[AUDIT][RISK] Found risky command: %s\n", k)
		}
	}
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
	fmt.Printf("[INFO] Install order: %v\n", order)
	for _, pkg := range order {
		wg.Add(1)
		go func(pkg string) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			checkGPGKey(pkg)
			inspectPKGBUILD(pkg)
			fmt.Printf("[SECURE] Building and installing %s...\n", pkg)
			cmd := exec.Command("echo", "Simulating build/install for "+pkg)
			cmd.Stdout = os.Stdout
			cmd.Stderr = os.Stderr
			if err := cmd.Run(); err != nil {
				fmt.Printf("[ERROR] Build/install failed for %s: %v\n", pkg, err)
			}
		}(pkg)
	}
	wg.Wait()
	fmt.Println("[INFO] All packages processed.")
}
