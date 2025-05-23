/*
Copyright © 2025 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

// installCmd represents the install command
var installCmd = &cobra.Command{
	Use:   "install [package]",
	Short: "Install a package from official, Chaotic-AUR, or AUR",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		pkg := args[0]
		err := InstallPackage(pkg)
		if err != nil {
			fmt.Printf("Failed to install '%s': %v\n", pkg, err)
		} else {
			fmt.Printf("Successfully installed '%s'!\n", pkg)
		}
	},
}

func init() {
	rootCmd.AddCommand(installCmd)
}
