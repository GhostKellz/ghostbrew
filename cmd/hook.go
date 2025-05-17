package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var hookCmd = &cobra.Command{
	Use:   "hook",
	Short: "Manage post-install hooks and plugins",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("[TODO] Hook system not yet implemented.")
	},
}

func init() {
	rootCmd.AddCommand(hookCmd)
}
