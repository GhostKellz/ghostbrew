package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var completionCmd = &cobra.Command{
	Use:   "completion [bash|zsh|fish]",
	Short: "Generate shell completions",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("[TODO] Shell completion for %s not yet implemented.\n", args[0])
	},
}

func init() {
	rootCmd.AddCommand(completionCmd)
}
