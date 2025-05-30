package cmd

import (
	"os"

	"gopkg.in/yaml.v3"
)

type Config struct {
	DefaultFlags    []string `yaml:"default_flags"`
	AurPath         string   `yaml:"aur_path"`
	IgnoredPackages []string `yaml:"ignored_packages"`
}

func LoadConfig() (*Config, error) {
	cfgPath := os.ExpandEnv("$HOME/.config/ghostbrew/config.yml")
	f, err := os.Open(cfgPath)
	if err != nil {
		return nil, err
	}
	defer f.Close()
	var cfg Config
	dec := yaml.NewDecoder(f)
	if err := dec.Decode(&cfg); err != nil {
		return nil, err
	}
	return &cfg, nil
}
