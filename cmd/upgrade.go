package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var upgradeCmd = &cobra.Command{
	Use:   "upgrade",
	Short: "Sync and upgrade all packages (official, Chaotic-AUR, AUR)",
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Println("[TODO] Upgrade logic not yet implemented. Will use pacman -Syu and AUR logic.")
	},
}

func init() {
	rootCmd.AddCommand(upgradeCmd)
}
