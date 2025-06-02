package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// Only one hookCmd should exist in the project!
var hookCmd = &cobra.Command{
	Use:   "hook",
	Short: "Manage post-install hooks and plugins",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("[TODO] Hook system: add/remove/list hooks and plugins.")
	},
}

// RunPostInstallHooks runs user-defined post-install hooks
func RunPostInstallHooks(pkg string) {
	// TODO: Load hooks from config and execute them
	fmt.Printf("[HOOK] Running post-install hooks for %s...\n", pkg)
	// Example: exec.Command("/usr/local/bin/ghostnotify.sh", pkg)
}

func init() {
	rootCmd.AddCommand(hookCmd)
}
