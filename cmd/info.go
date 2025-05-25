package cmd

import (
	"encoding/json"
	"net/http"
	"os"

	"github.com/fatih/color"
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
			color.Red("Failed to fetch info: %v", err)
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
			color.Red("No info found or failed to parse.")
			return
		}
		info := result.Results[0]
		color.Cyan("Name:        %s", info.Name)
		color.Yellow("Version:     %s", info.Version)
		color.White("Description: %s", info.Description)
		color.Green("Maintainer:  %s", info.Maintainer)
		color.Magenta("Votes:       %d", info.Votes)
		color.Blue("Popularity:  %.2f", info.Popularity)
		color.HiCyan("URL:         %s", info.URL)
	},
}

func init() {
	rootCmd.AddCommand(infoCmd)
}
