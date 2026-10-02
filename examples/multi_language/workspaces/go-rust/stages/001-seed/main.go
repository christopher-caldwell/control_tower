package main

import (
	"fmt"
	"os"
	"path/filepath"
)

func seed() error {
	data := filepath.Join(os.Getenv("CONTROL_TOWER_WORKSPACE"), "data")
	if err := os.MkdirAll(data, 0755); err != nil {
		return err
	}
	return os.WriteFile(filepath.Join(data, "number.txt"), []byte("21\n"), 0644)
}

func main() {
	if err := seed(); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
