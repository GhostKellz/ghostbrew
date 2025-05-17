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

// rootCmd represents the base command when called without any subcommands
var rootCmd = &cobra.Command{
	Use:   "ghostbrew",
	Short: "A brief description of your application",
	Long: `A longer description that spans multiple lines and likely contains
examples and usage of using your application. For example:

Cobra is a CLI library for Go that empowers applications.
This application is a tool to generate the needed files
to quickly create a Cobra application.`,
	// Uncomment the following line if your bare application
	// has an action associated with it:
	// Run: func(cmd *cobra.Command, args []string) { },
}

// Execute adds all child commands to the root command and sets flags appropriately.
// This is called by main.main(). It only needs to happen once to the rootCmd.
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

	err := rootCmd.Execute()
	if err != nil {
		os.Exit(1)
	}
}

func init() {
	// Here you will define your flags and configuration settings.
	// Cobra supports persistent flags, which, if defined here,
	// will be global for your application.

	// rootCmd.PersistentFlags().StringVar(&cfgFile, "config", "", "config file (default is $HOME/.ghostbrew.yaml)")
	rootCmd.PersistentFlags().StringVarP(&pkgToInstall, "sync", "S", "", "Install a package (like pacman -S)")
	rootCmd.PersistentFlags().StringVarP(&pkgToRemove, "remove", "R", "", "Remove a package (like pacman -R)")
	rootCmd.PersistentFlags().BoolVar(&removeCascade, "cascade", false, "Remove packages and all dependencies (like -Rs)")
	rootCmd.PersistentFlags().BoolVar(&removeUnneeded, "nosave", false, "Remove unneeded dependencies (like -Rns)")

	// Cobra also supports local flags, which will only run
	// when this action is called directly.
	rootCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
}
