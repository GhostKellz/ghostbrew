package cmd

import (
	"fmt"
	"os/exec"
)

// RemovePackage removes a package, optionally cascading and removing unneeded dependencies
func RemovePackage(pkg string, cascade, unneeded bool) error {
	args := []string{"-R"}
	if cascade {
		args = append(args, "s")
	}
	if unneeded {
		args = append(args, "n")
	}
	args = append(args, pkg)
	cmd := exec.Command("sudo", append([]string{"pacman"}, args...)...)
	cmd.Stdout = nil
	cmd.Stderr = nil
	err := cmd.Run()
	if err != nil {
		fmt.Printf("Error removing '%s': %v\n", pkg, err)
	}
	return err
}
