package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"

	"github.com/spf13/cobra"
)

var tapCmd = &cobra.Command{
	Use:   "tap <repo>",
	Short: "Add a private PKGBUILD repo (like brew tap)",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		repo := args[0]
		tapDir := filepath.Join(os.Getenv("HOME"), ".ghostbrew", "taps")
		os.MkdirAll(tapDir, 0755)
		fmt.Printf("Cloning %s into %s...\n", repo, tapDir)
		cmdGit := exec.Command("git", "clone", repo, filepath.Join(tapDir, filepath.Base(repo)))
		cmdGit.Stdout = os.Stdout
		cmdGit.Stderr = os.Stderr
		_ = cmdGit.Run()
	},
}

func init() {
	rootCmd.AddCommand(tapCmd)
}
