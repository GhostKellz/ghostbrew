package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"

	"github.com/spf13/cobra"
)

type AURResponse struct {
	Results []struct {
		Name        string `json:"Name"`
		Description string `json:"Description"`
		Version     string `json:"Version"`
	} `json:"results"`
}

var searchCmd = &cobra.Command{
	Use:   "search [package]",
	Short: "Search for packages in the AUR",
	Args:  cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		query := args[0]
		url := fmt.Sprintf("https://aur.archlinux.org/rpc/?v=5&type=search&arg=%s", query)

		resp, err := http.Get(url)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error fetching AUR data: %v\n", err)
			os.Exit(1)
		}
		defer resp.Body.Close()

		var data AURResponse
		if err := json.NewDecoder(resp.Body).Decode(&data); err != nil {
			fmt.Fprintf(os.Stderr, "Error parsing AUR response: %v\n", err)
			os.Exit(1)
		}

		if len(data.Results) == 0 {
			fmt.Println("No results found.")
			return
		}

		for _, pkg := range data.Results {
			fmt.Printf("📦 %s %s\n    %s\n\n", pkg.Name, pkg.Version, pkg.Description)
		}
	},
}

func init() {
	rootCmd.AddCommand(searchCmd)
}
