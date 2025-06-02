package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/exec"
	"strings"

	"github.com/spf13/cobra"
)

var upgradeCmd = &cobra.Command{
	Use:   "upgrade",
	Short: "Sync and upgrade all packages (official, Chaotic-AUR, AUR)",
	Run: func(cmd *cobra.Command, args []string) {
		// Parse flags
		noConfirm, _ := cmd.Flags().GetBool("no-confirm")
		aurOnly, _ := cmd.Flags().GetBool("aur-only")
		// Pacman system upgrade
		if !aurOnly {
			fmt.Println("Upgrading system packages with pacman...")
			pacmanArgs := []string{"-Syu"}
			if noConfirm {
				pacmanArgs = append(pacmanArgs, "--noconfirm")
			}
			cmdPacman := exec.Command("sudo", append([]string{"pacman"}, pacmanArgs...)...)
			cmdPacman.Stdout = os.Stdout
			cmdPacman.Stderr = os.Stderr
			_ = cmdPacman.Run()
		}
		// --- AUR upgrade logic ---
		fmt.Println("Checking for AUR package upgrades...")
		// 1. Get list of installed packages (pacman -Qm = foreign/AUR)
		out, err := exec.Command("pacman", "-Qm").Output()
		if err != nil {
			fmt.Println("Failed to list installed AUR packages:", err)
			return
		}
		var aurPkgs []string
		for _, line := range strings.Split(string(out), "\n") {
			if fields := strings.Fields(line); len(fields) > 0 {
				aurPkgs = append(aurPkgs, fields[0])
			}
		}
		if len(aurPkgs) == 0 {
			fmt.Println("No AUR packages installed.")
			return
		}
		// 2. Check for updates using AUR RPC API
		var toUpdate []string
		for _, pkg := range aurPkgs {
			resp, err := http.Get("https://aur.archlinux.org/rpc/?v=5&type=info&arg=" + pkg)
			if err != nil {
				continue
			}
			var result struct {
				Results []struct {
					Name    string
					Version string
				}
			}
			if err := json.NewDecoder(resp.Body).Decode(&result); err == nil && len(result.Results) > 0 {
				remoteVer := result.Results[0].Version
				// Get local version
				for _, line := range strings.Split(string(out), "\n") {
					fields := strings.Fields(line)
					if len(fields) > 1 && fields[0] == pkg && fields[1] != remoteVer {
						toUpdate = append(toUpdate, pkg)
					}
				}
			}
			resp.Body.Close()
		}
		if len(toUpdate) == 0 {
			fmt.Println("All AUR packages are up to date.")
			return
		}
		fmt.Printf("AUR packages to upgrade: %v\n", toUpdate)
		InstallPackages(toUpdate, InstallOptions{Parallel: 2})
	},
}

func init() {
	rootCmd.AddCommand(upgradeCmd)
	upgradeCmd.Flags().Bool("no-confirm", false, "Do not prompt for confirmation")
	upgradeCmd.Flags().Bool("aur-only", false, "Only upgrade AUR packages")
}
