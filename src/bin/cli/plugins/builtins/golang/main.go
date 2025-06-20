// Package main provides the cainome CLI executable for use with go generate.
//
// Usage in Go files:
//
//	//go:generate go run -mod=mod github.com/cartridge-gg/cainome/src/bin/cli/plugins/builtins/golang --golang --golang-package mycontract --output-dir ./bindings ./path/to/contract.json
package main

import (
	"fmt"
	"log"
	"os"
	"os/exec"
	"path/filepath"
	"runtime"
	"strings"
)

const (
	// Default binary name
	binaryName = "cainome"
	// Version to install if binary is not found
	defaultVersion = "latest"
)

func main() {
	if err := run(); err != nil {
		// In go generate context, prefix error for clarity
		if isGoGenerate() {
			log.Fatalf("go:generate cainome: %v", err)
		} else {
			log.Fatal(err)
		}
	}
}

func run() error {
	// Find or install the cainome binary
	binaryPath, err := findOrInstallBinary()
	if err != nil {
		return fmt.Errorf("failed to find or install cainome binary: %w", err)
	}

	// Debug logging if requested
	if debug := os.Getenv("CAINOME_DEBUG"); debug != "" {
		fmt.Fprintf(os.Stderr, "Using cainome binary: %s\n", binaryPath)
		fmt.Fprintf(os.Stderr, "Arguments: %v\n", os.Args[1:])
	}

	// Pass all arguments to the underlying cainome binary
	cmd := exec.Command(binaryPath, os.Args[1:]...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin

	// Set working directory to current directory
	cwd, err := os.Getwd()
	if err != nil {
		return fmt.Errorf("failed to get current directory: %w", err)
	}
	cmd.Dir = cwd

	// Run the command
	if err := cmd.Run(); err != nil {
		// Check if it's an exit error to provide more context
		if exitErr, ok := err.(*exec.ExitError); ok {
			return fmt.Errorf("cainome command failed with exit code %d", exitErr.ExitCode())
		}
		return fmt.Errorf("cainome command failed: %w", err)
	}

	return nil
}

func findOrInstallBinary() (string, error) {
	// Check for explicit binary path in environment
	if envPath := os.Getenv("CAINOME_BINARY"); envPath != "" {
		if _, err := os.Stat(envPath); err == nil {
			return envPath, nil
		}
		return "", fmt.Errorf("CAINOME_BINARY environment variable set to %s, but file not found", envPath)
	}

	// For local development, check if we're in the cainome repository
	if binaryPath := findLocalDevelopmentBinary(); binaryPath != "" {
		return binaryPath, nil
	}

	// Check if cainome is in PATH
	if path, err := exec.LookPath(binaryName); err == nil {
		return path, nil
	}

	// Check common installation locations
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}

	// Common binary locations
	possiblePaths := []string{
		filepath.Join(homeDir, ".cargo", "bin", binaryName),
		filepath.Join(homeDir, ".local", "bin", binaryName),
		filepath.Join("/usr", "local", "bin", binaryName),
		filepath.Join("/opt", "homebrew", "bin", binaryName),          // macOS with Homebrew on ARM
		filepath.Join("/usr", "local", "homebrew", "bin", binaryName), // macOS with Homebrew on Intel
	}

	// Add .exe extension on Windows
	if runtime.GOOS == "windows" {
		for i, path := range possiblePaths {
			possiblePaths[i] = path + ".exe"
		}
	}

	// Check each possible path
	for _, path := range possiblePaths {
		if _, err := os.Stat(path); err == nil {
			return path, nil
		}
	}

	// If not found and auto-install is disabled, return error
	if os.Getenv("CAINOME_NO_AUTO_INSTALL") != "" {
		return "", fmt.Errorf("cainome binary not found and auto-install is disabled (CAINOME_NO_AUTO_INSTALL is set)")
	}

	// If not found, try to install it
	fmt.Fprintln(os.Stderr, "cainome binary not found. Attempting to install...")
	return installBinary()
}

func findLocalDevelopmentBinary() string {
	// Check if we're running from within the cainome repository
	// by looking for Cargo.toml with cainome package
	cwd, err := os.Getwd()
	if err != nil {
		return ""
	}

	// Walk up the directory tree looking for the cainome repo root
	dir := cwd
	for {
		cargoPath := filepath.Join(dir, "Cargo.toml")
		if _, err := os.Stat(cargoPath); err == nil {
			// Check if this is the cainome Cargo.toml
			content, err := os.ReadFile(cargoPath)
			if err == nil && strings.Contains(string(content), `name = "cainome"`) {
				// Found the cainome repository root
				// Check for debug and release binaries
				targets := []string{
					filepath.Join(dir, "target", "release", binaryName),
					filepath.Join(dir, "target", "debug", binaryName),
				}

				if runtime.GOOS == "windows" {
					for i, path := range targets {
						targets[i] = path + ".exe"
					}
				}

				for _, target := range targets {
					if _, err := os.Stat(target); err == nil {
						if debug := os.Getenv("CAINOME_DEBUG"); debug != "" {
							fmt.Fprintf(os.Stderr, "Found local development binary: %s\n", target)
						}
						return target
					}
				}
			}
		}

		// Move to parent directory
		parent := filepath.Dir(dir)
		if parent == dir {
			// Reached the root
			break
		}
		dir = parent
	}

	return ""
}

func installBinary() (string, error) {
	// Check if cargo is available
	if _, err := exec.LookPath("cargo"); err != nil {
		return "", fmt.Errorf("cargo not found. Please install Rust and Cargo from https://rustup.rs/")
	}

	// Install cainome using cargo
	fmt.Fprintln(os.Stderr, "Installing cainome with cargo...")

	// Determine installation command based on environment
	args := []string{"install"}

	// Check if we should install from a specific source
	if source := os.Getenv("CAINOME_INSTALL_SOURCE"); source != "" {
		switch source {
		case "crates.io":
			args = append(args, "cainome")
		case "github", "git":
			args = append(args, "--git", "https://github.com/cartridge-gg/cainome")
		default:
			// Assume it's a path
			args = append(args, "--path", source)
		}
	} else {
		// Default to github
		args = append(args, "--git", "https://github.com/cartridge-gg/cainome")
	}

	// Always specify the binary name
	args = append(args, "--bin", "cainome")

	// Check if we should install a specific version
	if version := os.Getenv("CAINOME_VERSION"); version != "" && version != "latest" {
		// Only add tag for git installations
		if strings.Contains(strings.Join(args, " "), "--git") {
			args = append(args, "--tag", version)
		} else if strings.Contains(strings.Join(args, " "), "cainome") && !strings.Contains(strings.Join(args, " "), "--path") {
			// For crates.io, modify the package name
			for i, arg := range args {
				if arg == "cainome" {
					args[i] = fmt.Sprintf("cainome@%s", version)
					break
				}
			}
		}
	}

	installCmd := exec.Command("cargo", args...)
	installCmd.Stdout = os.Stdout
	installCmd.Stderr = os.Stderr

	if err := installCmd.Run(); err != nil {
		return "", fmt.Errorf("failed to install cainome: %w", err)
	}

	// Find the installed binary
	homeDir, err := os.UserHomeDir()
	if err != nil {
		return "", fmt.Errorf("failed to get home directory: %w", err)
	}

	binaryPath := filepath.Join(homeDir, ".cargo", "bin", binaryName)
	if runtime.GOOS == "windows" {
		binaryPath += ".exe"
	}

	if _, err := os.Stat(binaryPath); err != nil {
		return "", fmt.Errorf("cainome installed but binary not found at expected location: %s", binaryPath)
	}

	fmt.Fprintf(os.Stderr, "Successfully installed cainome at: %s\n", binaryPath)
	return binaryPath, nil
}

// Helper function to detect if we're running in go generate context
func isGoGenerate() bool {
	// Check for common go generate environment variables
	return os.Getenv("GOPACKAGE") != "" || os.Getenv("GOFILE") != "" || os.Getenv("GOLINE") != ""
}
