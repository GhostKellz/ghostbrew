package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var auditCmd = &cobra.Command{
	Use:   "audit <pkg>",
	Short: "Inspect a PKGBUILD for risky commands before installing",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		pkg := args[0]
		fmt.Printf("Auditing PKGBUILD for %s...\n", pkg)
		// TODO: Implement PKGBUILD fetch and highlight risky commands
		fmt.Println("[TODO] PKGBUILD inspection not yet implemented.")
	},
}

func init() {
	rootCmd.AddCommand(auditCmd)
}
