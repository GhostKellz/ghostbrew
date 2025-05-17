package cmd

import (
	"fmt"
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
		// TODO: Query installed AUR packages and upgrade them
		fmt.Println("[TODO] AUR upgrade logic not yet implemented.")
	},
}

func init() {
	rootCmd.AddCommand(upgradeCmd)
	upgradeCmd.Flags().Bool("no-confirm", false, "Do not prompt for confirmation")
	upgradeCmd.Flags().Bool("aur-only", false, "Only upgrade AUR packages")
}
