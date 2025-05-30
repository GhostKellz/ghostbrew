/*
Copyright © 2025 NAME HERE <EMAIL ADDRESS>
*/
package cmd

import (
	"fmt"
	"os"

	"github.com/spf13/cobra"
)

var pkgToInstall string
var pkgToRemove string
var removeCascade bool
var removeUnneeded bool

var rootCmd = &cobra.Command{
	Use:   "ghostbrew",
	Short: "AUR helper and package manager for Arch Linux",
	Long:  `ghostbrew is a modern AUR helper and package manager for Arch Linux, supporting official, Chaotic-AUR, and AUR packages.`,
}

func Execute() {
	if pkgToInstall != "" {
		err := InstallPackage(pkgToInstall)
		if err != nil {
			fmt.Printf("Failed to install '%s': %v\n", pkgToInstall, err)
			os.Exit(1)
		} else {
			fmt.Printf("Successfully installed '%s'!\n", pkgToInstall)
			os.Exit(0)
		}
	}
	if pkgToRemove != "" {
		err := RemovePackage(pkgToRemove, removeCascade, removeUnneeded)
		if err != nil {
			fmt.Printf("Failed to remove '%s': %v\n", pkgToRemove, err)
			os.Exit(1)
		} else {
			fmt.Printf("Successfully removed '%s'!\n", pkgToRemove)
			os.Exit(0)
		}
	}
	if err := rootCmd.Execute(); err != nil {
		os.Exit(1)
	}
}

func init() {
	rootCmd.PersistentFlags().StringVarP(&pkgToInstall, "sync", "S", "", "Install a package (like pacman -S)")
	rootCmd.PersistentFlags().StringVarP(&pkgToRemove, "remove", "R", "", "Remove a package (like pacman -R)")
	rootCmd.PersistentFlags().BoolVar(&removeCascade, "cascade", false, "Remove packages and all dependencies (like -Rs)")
	rootCmd.PersistentFlags().BoolVar(&removeUnneeded, "nosave", false, "Remove unneeded dependencies (like -Rns)")
}
