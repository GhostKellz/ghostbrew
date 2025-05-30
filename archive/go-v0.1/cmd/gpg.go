package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var gpgCmd = &cobra.Command{
	Use:   "gpg",
	Short: "Handle GPG key import and troubleshooting",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("[TODO] Smart GPG key handling not yet implemented.")
	},
}

func init() {
	rootCmd.AddCommand(gpgCmd)
}
