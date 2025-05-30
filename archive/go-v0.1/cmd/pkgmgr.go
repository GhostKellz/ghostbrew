package cmd

import (
	"bufio"
	"bytes"
	"fmt"
	"os/exec"
	"strings"
)

// IsInOfficialRepos checks if a package is available in the official repos
func IsInOfficialRepos(pkg string) bool {
	cmd := exec.Command("pacman", "-Si", pkg)
	return cmd.Run() == nil
}

// IsInChaoticAUR checks if a package is available in Chaotic-AUR
func IsInChaoticAUR(pkg string) bool {
	cmd := exec.Command("pacman", "-Sl", "chaotic-aur")
	out, err := cmd.Output()
	if err != nil {
		return false
	}
	scanner := bufio.NewScanner(bytes.NewReader(out))
	for scanner.Scan() {
		fields := strings.Fields(scanner.Text())
		if len(fields) > 1 && fields[1] == pkg {
			return true
		}
	}
	return false
}

// IsInAUR checks if a package is available in the AUR using ghostbrew as a backend
func IsInAUR(pkg string) bool {
	cmd := exec.Command("ghostbrew", "aur", "info", pkg)
	return cmd.Run() == nil
}

// InstallPackage installs a package from the best available source
func InstallPackage(pkg string) error {
	fmt.Printf("Searching for '%s'...\n", pkg)
	if IsInOfficialRepos(pkg) {
		fmt.Println("Found in official repos. Installing with pacman...")
		return PacmanInstall(pkg)
	}
	if IsInChaoticAUR(pkg) {
		fmt.Println("Found in Chaotic-AUR. Installing with pacman...")
		return PacmanInstall(pkg)
	}
	if IsInAUR(pkg) {
		fmt.Println("Found in AUR. Building and installing with ghostbrew...")
		return BuildAndInstallAUR(pkg)
	}
	return fmt.Errorf("package '%s' not found in official repos, Chaotic-AUR, or AUR", pkg)
}

// PacmanInstall installs a package using pacman
func PacmanInstall(pkg string) error {
	cmd := exec.Command("sudo", "pacman", "-S", "--noconfirm", pkg)
	cmd.Stdout = nil
	cmd.Stderr = nil
	err := cmd.Run()
	if err != nil {
		fmt.Printf("Error installing '%s' with pacman: %v\n", pkg, err)
	}
	return err
}

// BuildAndInstallAUR builds and installs a package from AUR using ghostbrew
func BuildAndInstallAUR(pkg string) error {
	cmd := exec.Command("ghostbrew", "aur", "install", pkg)
	cmd.Stdout = nil
	cmd.Stderr = nil
	err := cmd.Run()
	if err != nil {
		fmt.Printf("Error building/installing '%s' from AUR: %v\n", pkg, err)
	}
	return err
}
