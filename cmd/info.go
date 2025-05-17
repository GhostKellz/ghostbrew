package cmd

import (
	"encoding/json"
	"fmt"
	"net/http"
	"os"

	"github.com/spf13/cobra"
)

var infoCmd = &cobra.Command{
	Use:   "info <pkg>",
	Short: "Show full AUR info for a package",
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		pkg := args[0]
		resp, err := http.Get("https://aur.archlinux.org/rpc/?v=5&type=info&arg=" + pkg)
		if err != nil {
			fmt.Println("Failed to fetch info:", err)
			os.Exit(1)
		}
		defer resp.Body.Close()
		var result struct {
			Results []struct {
				Name           string
				Version        string
				Description    string
				Maintainer     string
				URL            string
				Votes          int
				Popularity     float64
				FirstSubmitted int64
				LastModified   int64
			}
		}
		if err := json.NewDecoder(resp.Body).Decode(&result); err != nil || len(result.Results) == 0 {
			fmt.Println("No info found or failed to parse.")
			return
		}
		info := result.Results[0]
		fmt.Printf("Name: %s\nVersion: %s\nDescription: %s\nMaintainer: %s\nVotes: %d\nPopularity: %.2f\nURL: %s\n", info.Name, info.Version, info.Description, info.Maintainer, info.Votes, info.Popularity, info.URL)
	},
}

func init() {
	rootCmd.AddCommand(infoCmd)
}
