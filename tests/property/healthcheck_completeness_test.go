// Copyright 2024 SDV Parking Demo System
// Property-based test for Health Check Configuration Completeness.
//
// **Feature: project-foundation, Property 2: Health Check Configuration Completeness**
//
// This test verifies that:
// *For any* service defined in the Podman Compose configuration, there SHALL exist
// a corresponding health check configuration with test command, interval, timeout,
// and retries specified.
//
// **Validates: Requirements 3.6**

package property

import (
	"os"
	"path/filepath"
	"testing"

	"pgregory.net/rapid"
	"gopkg.in/yaml.v3"
)

// ComposeFile represents the structure of a Podman/Docker Compose file
type ComposeFile struct {
	Version  string                    `yaml:"version"`
	Services map[string]ComposeService `yaml:"services"`
}

// ComposeService represents a service definition in the compose file
type ComposeService struct {
	Image       string            `yaml:"image"`
	Build       interface{}       `yaml:"build"`
	Ports       []string          `yaml:"ports"`
	Volumes     []string          `yaml:"volumes"`
	Environment []string          `yaml:"environment"`
	Command     interface{}       `yaml:"command"`
	DependsOn   interface{}       `yaml:"depends_on"`
	HealthCheck *HealthCheckConfig `yaml:"healthcheck"`
	Restart     string            `yaml:"restart"`
	Networks    []string          `yaml:"networks"`
}

// HealthCheckConfig represents the health check configuration
type HealthCheckConfig struct {
	Test        interface{} `yaml:"test"`
	Interval    string      `yaml:"interval"`
	Timeout     string      `yaml:"timeout"`
	Retries     int         `yaml:"retries"`
	StartPeriod string      `yaml:"start_period"`
}

// ServiceHealthInfo holds parsed health check information for a service
type ServiceHealthInfo struct {
	ServiceName     string
	HasHealthCheck  bool
	HasTest         bool
	HasInterval     bool
	HasTimeout      bool
	HasRetries      bool
	HasStartPeriod  bool
	HealthCheck     *HealthCheckConfig
}

// parseComposeFile parses a Podman/Docker Compose YAML file
func parseComposeFile(path string) (*ComposeFile, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, err
	}

	var compose ComposeFile
	if err := yaml.Unmarshal(data, &compose); err != nil {
		return nil, err
	}

	return &compose, nil
}

// analyzeServiceHealth analyzes health check configuration for a service
func analyzeServiceHealth(serviceName string, service ComposeService) *ServiceHealthInfo {
	info := &ServiceHealthInfo{
		ServiceName:    serviceName,
		HasHealthCheck: service.HealthCheck != nil,
	}

	if service.HealthCheck != nil {
		info.HealthCheck = service.HealthCheck
		info.HasTest = service.HealthCheck.Test != nil
		info.HasInterval = service.HealthCheck.Interval != ""
		info.HasTimeout = service.HealthCheck.Timeout != ""
		info.HasRetries = service.HealthCheck.Retries > 0
		info.HasStartPeriod = service.HealthCheck.StartPeriod != ""
	}

	return info
}

// findComposeFiles finds all compose files in the infra directory
func findComposeFiles(projectRoot string) ([]string, error) {
	composeDir := filepath.Join(projectRoot, "infra", "compose")
	var composeFiles []string

	err := filepath.Walk(composeDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return err
		}
		if !info.IsDir() {
			name := info.Name()
			// Match common compose file patterns
			if name == "podman-compose.yml" || name == "podman-compose.yaml" ||
				name == "docker-compose.yml" || name == "docker-compose.yaml" ||
				name == "compose.yml" || name == "compose.yaml" {
				composeFiles = append(composeFiles, path)
			}
		}
		return nil
	})

	return composeFiles, err
}

// TestHealthCheckConfigurationCompleteness is the main property-based test
// **Feature: project-foundation, Property 2: Health Check Configuration Completeness**
// **Validates: Requirements 3.6**
func TestHealthCheckConfigurationCompleteness(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	composeFiles, err := findComposeFiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find compose files: %v", err)
	}

	if len(composeFiles) == 0 {
		t.Fatal("No compose files found in infra/compose/ directory")
	}

	// Collect all services from all compose files
	var allServices []struct {
		file    string
		name    string
		service ComposeService
	}

	for _, composeFile := range composeFiles {
		compose, err := parseComposeFile(composeFile)
		if err != nil {
			t.Fatalf("Failed to parse compose file %s: %v", composeFile, err)
		}

		for name, service := range compose.Services {
			allServices = append(allServices, struct {
				file    string
				name    string
				service ComposeService
			}{composeFile, name, service})
		}
	}

	if len(allServices) == 0 {
		t.Fatal("No services found in compose files")
	}

	t.Logf("Found %d services across %d compose files", len(allServices), len(composeFiles))

	rapid.Check(t, func(rt *rapid.T) {
		// Select a random service to test
		idx := rapid.IntRange(0, len(allServices)-1).Draw(rt, "service_index")
		svc := allServices[idx]

		info := analyzeServiceHealth(svc.name, svc.service)

		// Property: Every service must have a health check
		if !info.HasHealthCheck {
			rt.Fatalf("Service '%s' in %s: Missing health check configuration",
				svc.name, svc.file)
		}

		// Property: Health check must have a test command
		if !info.HasTest {
			rt.Fatalf("Service '%s' in %s: Health check missing 'test' command",
				svc.name, svc.file)
		}

		// Property: Health check must have interval
		if !info.HasInterval {
			rt.Fatalf("Service '%s' in %s: Health check missing 'interval'",
				svc.name, svc.file)
		}

		// Property: Health check must have timeout
		if !info.HasTimeout {
			rt.Fatalf("Service '%s' in %s: Health check missing 'timeout'",
				svc.name, svc.file)
		}

		// Property: Health check must have retries
		if !info.HasRetries {
			rt.Fatalf("Service '%s' in %s: Health check missing 'retries' (or retries <= 0)",
				svc.name, svc.file)
		}
	})
}

// TestAllServicesHaveHealthChecks tests all services deterministically
func TestAllServicesHaveHealthChecks(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	composeFiles, err := findComposeFiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find compose files: %v", err)
	}

	if len(composeFiles) == 0 {
		t.Fatal("No compose files found in infra/compose/ directory")
	}

	for _, composeFile := range composeFiles {
		relPath, _ := filepath.Rel(projectRoot, composeFile)
		
		compose, err := parseComposeFile(composeFile)
		if err != nil {
			t.Fatalf("Failed to parse compose file %s: %v", composeFile, err)
		}

		for serviceName, service := range compose.Services {
			t.Run(relPath+"/"+serviceName, func(t *testing.T) {
				info := analyzeServiceHealth(serviceName, service)

				// Check health check exists
				if !info.HasHealthCheck {
					t.Errorf("Missing health check configuration")
					return
				}

				// Check required fields
				var missing []string
				if !info.HasTest {
					missing = append(missing, "test")
				}
				if !info.HasInterval {
					missing = append(missing, "interval")
				}
				if !info.HasTimeout {
					missing = append(missing, "timeout")
				}
				if !info.HasRetries {
					missing = append(missing, "retries")
				}

				if len(missing) > 0 {
					t.Errorf("Health check missing required fields: %v", missing)
				}

				// Log health check details
				if info.HealthCheck != nil {
					t.Logf("✓ Health check: interval=%s, timeout=%s, retries=%d",
						info.HealthCheck.Interval,
						info.HealthCheck.Timeout,
						info.HealthCheck.Retries)
				}
			})
		}
	}
}

// TestHealthCheckFieldValidation tests that health check fields have valid values
func TestHealthCheckFieldValidation(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	composeFiles, err := findComposeFiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find compose files: %v", err)
	}

	for _, composeFile := range composeFiles {
		compose, err := parseComposeFile(composeFile)
		if err != nil {
			t.Fatalf("Failed to parse compose file %s: %v", composeFile, err)
		}

		for serviceName, service := range compose.Services {
			if service.HealthCheck == nil {
				continue
			}

			t.Run(serviceName, func(t *testing.T) {
				hc := service.HealthCheck

				// Validate interval format (should be like "10s", "1m", etc.)
				if hc.Interval != "" {
					if !isValidDuration(hc.Interval) {
						t.Errorf("Invalid interval format: %s", hc.Interval)
					}
				}

				// Validate timeout format
				if hc.Timeout != "" {
					if !isValidDuration(hc.Timeout) {
						t.Errorf("Invalid timeout format: %s", hc.Timeout)
					}
				}

				// Validate retries is positive
				if hc.Retries <= 0 {
					t.Errorf("Retries should be positive, got: %d", hc.Retries)
				}

				// Validate start_period format if present
				if hc.StartPeriod != "" {
					if !isValidDuration(hc.StartPeriod) {
						t.Errorf("Invalid start_period format: %s", hc.StartPeriod)
					}
				}
			})
		}
	}
}

// isValidDuration checks if a string is a valid duration format
func isValidDuration(s string) bool {
	if len(s) < 2 {
		return false
	}
	
	// Check for valid suffixes
	validSuffixes := []string{"s", "m", "h", "ms", "us", "ns"}
	for _, suffix := range validSuffixes {
		if len(s) > len(suffix) && s[len(s)-len(suffix):] == suffix {
			// Check that the prefix is numeric
			prefix := s[:len(s)-len(suffix)]
			for _, c := range prefix {
				if c < '0' || c > '9' {
					return false
				}
			}
			return true
		}
	}
	return false
}

// TestHealthCheckTestCommandFormat tests that test commands are properly formatted
func TestHealthCheckTestCommandFormat(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	composeFiles, err := findComposeFiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find compose files: %v", err)
	}

	for _, composeFile := range composeFiles {
		compose, err := parseComposeFile(composeFile)
		if err != nil {
			t.Fatalf("Failed to parse compose file %s: %v", composeFile, err)
		}

		for serviceName, service := range compose.Services {
			if service.HealthCheck == nil || service.HealthCheck.Test == nil {
				continue
			}

			t.Run(serviceName, func(t *testing.T) {
				test := service.HealthCheck.Test

				// Test can be either a string or array
				switch v := test.(type) {
				case string:
					if v == "" {
						t.Error("Health check test command is empty")
					}
					t.Logf("✓ Test command (string): %s", truncateString(v, 50))
				case []interface{}:
					if len(v) == 0 {
						t.Error("Health check test command array is empty")
					}
					// First element should be CMD, CMD-SHELL, or NONE
					if len(v) > 0 {
						first, ok := v[0].(string)
						if !ok {
							t.Error("First element of test array should be a string")
						} else if first != "CMD" && first != "CMD-SHELL" && first != "NONE" {
							t.Errorf("First element should be CMD, CMD-SHELL, or NONE, got: %s", first)
						}
					}
					t.Logf("✓ Test command (array): %v", v)
				default:
					t.Errorf("Unexpected test command type: %T", test)
				}
			})
		}
	}
}

// truncateString truncates a string to maxLen characters
func truncateString(s string, maxLen int) string {
	if len(s) <= maxLen {
		return s
	}
	return s[:maxLen] + "..."
}

// TestComposeFileStructure tests the overall structure of compose files
func TestComposeFileStructure(t *testing.T) {
	projectRoot, err := findProjectRoot()
	if err != nil {
		t.Fatalf("Failed to find project root: %v", err)
	}

	composeFiles, err := findComposeFiles(projectRoot)
	if err != nil {
		t.Fatalf("Failed to find compose files: %v", err)
	}

	if len(composeFiles) == 0 {
		t.Fatal("No compose files found")
	}

	for _, composeFile := range composeFiles {
		relPath, _ := filepath.Rel(projectRoot, composeFile)
		t.Run(relPath, func(t *testing.T) {
			compose, err := parseComposeFile(composeFile)
			if err != nil {
				t.Fatalf("Failed to parse: %v", err)
			}

			// Check version is specified
			if compose.Version == "" {
				t.Log("⚠ No version specified (optional in newer compose)")
			} else {
				t.Logf("✓ Version: %s", compose.Version)
			}

			// Check services exist
			if len(compose.Services) == 0 {
				t.Error("No services defined")
			} else {
				t.Logf("✓ Services: %d", len(compose.Services))
			}

			// List all services
			for name := range compose.Services {
				t.Logf("  - %s", name)
			}
		})
	}
}
