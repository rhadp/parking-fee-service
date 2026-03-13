// Package setup contains integration tests that verify the project structure
// and build system for the SDV Parking Demo monorepo (spec 01_project_setup).
package setup

import (
	"os"
	"path/filepath"
	"testing"
)

// repoRoot returns the absolute path to the repository root.
// Tests are in tests/setup/, so the root is two levels up.
func repoRoot(t *testing.T) string {
	t.Helper()
	// Walk up from the test file's directory to find the repo root.
	wd, err := os.Getwd()
	if err != nil {
		t.Fatalf("failed to get working directory: %v", err)
	}
	root := filepath.Join(wd, "..", "..")
	abs, err := filepath.Abs(root)
	if err != nil {
		t.Fatalf("failed to resolve repo root: %v", err)
	}
	return abs
}

// isDir checks whether a path exists and is a directory.
func isDir(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return info.IsDir()
}

// fileExists checks whether a path exists and is a regular file.
func fileExists(path string) bool {
	info, err := os.Stat(path)
	if err != nil {
		return false
	}
	return !info.IsDir()
}

// TS-01-1: Top-Level Directories Exist
// Requirement: 01-REQ-1.1
func TestTopLevelDirectories(t *testing.T) {
	root := repoRoot(t)

	dirs := []string{"rhivos", "backend", "android", "mobile", "mock", "proto", "deployments", "tests"}
	for _, dir := range dirs {
		path := filepath.Join(root, dir)
		if !isDir(path) {
			t.Errorf("expected top-level directory %q to exist", dir)
		}
	}
}

// TS-01-2: Rust Component Subdirectories Exist
// Requirement: 01-REQ-1.2
func TestRustComponentDirs(t *testing.T) {
	root := repoRoot(t)

	dirs := []string{"locking-service", "cloud-gateway-client", "update-service", "parking-operator-adaptor", "mock-sensors"}
	for _, dir := range dirs {
		path := filepath.Join(root, "rhivos", dir)
		if !isDir(path) {
			t.Errorf("expected Rust component directory rhivos/%s to exist", dir)
		}
	}
}

// TS-01-3: Go Backend Subdirectories Exist
// Requirement: 01-REQ-1.3
func TestGoBackendDirs(t *testing.T) {
	root := repoRoot(t)

	dirs := []string{"parking-fee-service", "cloud-gateway"}
	for _, dir := range dirs {
		path := filepath.Join(root, "backend", dir)
		if !isDir(path) {
			t.Errorf("expected Go backend directory backend/%s to exist", dir)
		}
	}
}

// TS-01-4: Mock CLI Subdirectories Exist
// Requirement: 01-REQ-1.4
func TestMockCliDirs(t *testing.T) {
	root := repoRoot(t)

	dirs := []string{"parking-app-cli", "companion-app-cli", "parking-operator"}
	for _, dir := range dirs {
		path := filepath.Join(root, "mock", dir)
		if !isDir(path) {
			t.Errorf("expected mock CLI directory mock/%s to exist", dir)
		}
	}
}

// TS-01-5: AAOS Placeholder Exists
// Requirement: 01-REQ-1.5
func TestAaosPlaceholder(t *testing.T) {
	root := repoRoot(t)

	if !isDir(filepath.Join(root, "android")) {
		t.Error("expected android/ directory to exist")
	}
	if !fileExists(filepath.Join(root, "android", "README.md")) {
		t.Error("expected android/README.md to exist")
	}
}

// TS-01-6: Flutter Placeholder Exists
// Requirement: 01-REQ-1.6
func TestFlutterPlaceholder(t *testing.T) {
	root := repoRoot(t)

	if !isDir(filepath.Join(root, "mobile")) {
		t.Error("expected mobile/ directory to exist")
	}
	if !fileExists(filepath.Join(root, "mobile", "README.md")) {
		t.Error("expected mobile/README.md to exist")
	}
}
