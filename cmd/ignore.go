package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var ignoreCmd = &cobra.Command{
	Use:   "ignore <pkg>",
	Short: "Add a package to the ignore list (lock/ignore)",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("[TODO] Add %s to ignore list in config. Not yet implemented.\n", args[0])
	},
}

func init() {
	rootCmd.AddCommand(ignoreCmd)
}
