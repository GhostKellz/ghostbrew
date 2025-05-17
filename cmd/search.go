/*
Copyright © 2025 Christopher Kelley <ckelley@ghostkellz.sh>
*/
package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"github.com/manifoldco/promptui"
	"github.com/spf13/cobra"
)

// searchCmd represents the search command
var searchCmd = &cobra.Command{
	Use:   "search",
	Short: "A brief description of your command",
	Long: `A longer description that spans multiple lines and likely contains examples
and usage of using your command. For example:

Cobra is a CLI library for Go that empowers applications.
This application is a tool to generate the needed files
to quickly create a Cobra application.`,
	Run: func(cmd *cobra.Command, args []string) {
		if len(args) == 0 {
			fmt.Println("Please provide a search term.")
			return
		}
		searchTerm := strings.Join(args, " ")
		resp, err := http.Get("https://aur.archlinux.org/rpc/?v=5&type=search&arg=" + searchTerm)
		if err != nil {
			fmt.Println("Failed to search AUR:", err)
			return
		}
		defer resp.Body.Close()
		var result struct {
			Results []struct {
				Name        string
				Description string
				Version     string
				Maintainer  string
			}
		}
		if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
			fmt.Println("Failed to parse AUR response:", err)
			return
		}
		if len(result.Results) == 0 {
			fmt.Println("No results found.")
			return
		}
		items := make([]string, len(result.Results))
		for i, r := range result.Results {
			items[i] = fmt.Sprintf("%s (%s) - %s", r.Name, r.Version, r.Description)
		}
		prompt := promptui.Select{
			Label: "Select package to install",
			Items: items,
		}
		idx, _, err := prompt.Run()
		if err != nil {
			fmt.Printf("Prompt failed %v\n", err)
			return
		}
		selected := result.Results[idx].Name
		fmt.Printf("You selected %q. Installing...\n", selected)
		// Dependency resolution, GPG, PKGBUILD inspection, and parallel install handled in InstallPackages
		InstallPackages([]string{selected}, InstallOptions{Parallel: 2})
	},
}

func init() {
	rootCmd.AddCommand(searchCmd)

	// Here you will define your flags and configuration settings.

	// Cobra supports Persistent Flags which will work for this command
	// and all subcommands, e.g.:
	// searchCmd.PersistentFlags().String("foo", "", "A help for foo")

	// Cobra supports local flags which will only run when this command
	// is called directly, e.g.:
	// searchCmd.Flags().BoolP("toggle", "t", false, "Help message for toggle")
}
