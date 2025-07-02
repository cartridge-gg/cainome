package main

import (
	"os"
	"path/filepath"
	"runtime"
	"testing"
)

func TestFindLocalDevelopmentBinary(t *testing.T) {
	// Save current directory
	originalWd, err := os.Getwd()
	if err != nil {
		t.Fatal(err)
	}
	defer os.Chdir(originalWd)

	// Test from within cmd/cainome directory
	binaryPath := findLocalDevelopmentBinary()

	// We should find a binary if we're in the cainome repo
	// This test will pass when run from within the cainome repository
	t.Logf("Found binary path: %s", binaryPath)

	// Test that the function returns empty string when not in repo
	tempDir := t.TempDir()
	if err := os.Chdir(tempDir); err != nil {
		t.Fatal(err)
	}

	binaryPath = findLocalDevelopmentBinary()
	if binaryPath != "" {
		t.Errorf("Expected empty path when not in cainome repo, got: %s", binaryPath)
	}
}

func TestIsGoGenerate(t *testing.T) {
	tests := []struct {
		name     string
		envVars  map[string]string
		expected bool
	}{
		{
			name:     "No go generate vars",
			envVars:  map[string]string{},
			expected: false,
		},
		{
			name: "With GOPACKAGE",
			envVars: map[string]string{
				"GOPACKAGE": "main",
			},
			expected: true,
		},
		{
			name: "With GOFILE",
			envVars: map[string]string{
				"GOFILE": "main.go",
			},
			expected: true,
		},
		{
			name: "With GOLINE",
			envVars: map[string]string{
				"GOLINE": "42",
			},
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			// Clear relevant env vars
			os.Unsetenv("GOPACKAGE")
			os.Unsetenv("GOFILE")
			os.Unsetenv("GOLINE")

			// Set test env vars
			for k, v := range tt.envVars {
				os.Setenv(k, v)
				defer os.Unsetenv(k)
			}

			result := isGoGenerate()
			if result != tt.expected {
				t.Errorf("isGoGenerate() = %v, want %v", result, tt.expected)
			}
		})
	}
}

func TestBinaryName(t *testing.T) {
	expectedName := "cainome"
	if runtime.GOOS == "windows" {
		// On Windows, we'd expect .exe extension in actual paths
		// but the constant should remain without extension
	}

	if binaryName != expectedName {
		t.Errorf("Binary name = %s, want %s", binaryName, expectedName)
	}
}

func TestFindOrInstallBinaryWithEnvVar(t *testing.T) {
	// Create a temporary file to act as binary
	tempDir := t.TempDir()
	tempBinary := filepath.Join(tempDir, "cainome")
	if runtime.GOOS == "windows" {
		tempBinary += ".exe"
	}

	// Create the file
	if err := os.WriteFile(tempBinary, []byte("test"), 0755); err != nil {
		t.Fatal(err)
	}

	// Set environment variable
	os.Setenv("CAINOME_BINARY", tempBinary)
	defer os.Unsetenv("CAINOME_BINARY")

	// Test finding binary
	path, err := findOrInstallBinary()
	if err != nil {
		t.Errorf("findOrInstallBinary() error = %v", err)
	}

	if path != tempBinary {
		t.Errorf("findOrInstallBinary() = %s, want %s", path, tempBinary)
	}

	// Test with non-existent path
	os.Setenv("CAINOME_BINARY", "/non/existent/path")
	_, err = findOrInstallBinary()
	if err == nil {
		t.Error("Expected error for non-existent CAINOME_BINARY path")
	}
}
