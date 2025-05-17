package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"sync"
)

type InstallOptions struct {
	Parallel int // Number of parallel jobs
}

func InstallPackages(pkgs []string, opts InstallOptions) {
	var wg sync.WaitGroup
	sem := make(chan struct{}, opts.Parallel)
	for _, pkg := range pkgs {
		wg.Add(1)
		go func(pkg string) {
			defer wg.Done()
			sem <- struct{}{}
			defer func() { <-sem }()
			fmt.Printf("Building and installing %s...\n", pkg)
			cmd := exec.Command("echo", "Simulating build/install for "+pkg)
			cmd.Stdout = os.Stdout
			cmd.Stderr = os.Stderr
			_ = cmd.Run()
		}(pkg)
	}
	wg.Wait()
}
