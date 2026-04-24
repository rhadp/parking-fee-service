package setup_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-23: compose.yml defines NATS and Kuksa services
// Requirement: 01-REQ-7.1
func TestComposeDefinesServices(t *testing.T) {
	root := repoRoot(t)

	composePath := filepath.Join(root, "deployments", "compose.yml")

	if !pathExists(composePath) {
		t.Fatalf("expected %s to exist", composePath)
	}

	data, err := os.ReadFile(composePath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", composePath, err)
	}
	content := string(data)

	t.Run("nats service defined", func(t *testing.T) {
		if !strings.Contains(content, "nats") {
			t.Errorf("compose.yml should define a nats service")
		}
	})

	t.Run("nats port 4222", func(t *testing.T) {
		if !strings.Contains(content, "4222") {
			t.Errorf("compose.yml should expose NATS on port 4222")
		}
	})

	t.Run("kuksa-databroker service defined", func(t *testing.T) {
		// Check for kuksa-databroker or kuksa_databroker or kuksa
		if !strings.Contains(content, "kuksa") {
			t.Errorf("compose.yml should define a kuksa-databroker service")
		}
	})

	t.Run("kuksa port 55556", func(t *testing.T) {
		if !strings.Contains(content, "55556") {
			t.Errorf("compose.yml should expose Kuksa Databroker on port 55556")
		}
	})
}

// TS-01-24: NATS configuration file exists
// Requirement: 01-REQ-7.2
func TestNATSConfigExists(t *testing.T) {
	root := repoRoot(t)

	confPath := filepath.Join(root, "deployments", "nats", "nats-server.conf")

	if !pathExists(confPath) {
		t.Fatalf("expected %s to exist", confPath)
	}

	info, err := os.Stat(confPath)
	if err != nil {
		t.Fatalf("failed to stat %s: %v", confPath, err)
	}

	if info.Size() == 0 {
		t.Errorf("nats-server.conf should not be empty")
	}
}

// TS-01-25: VSS overlay defines custom signals
// Requirement: 01-REQ-7.3
func TestVSSOverlaySignals(t *testing.T) {
	root := repoRoot(t)

	overlayPath := filepath.Join(root, "deployments", "vss-overlay.json")

	if !pathExists(overlayPath) {
		t.Fatalf("expected %s to exist", overlayPath)
	}

	data, err := os.ReadFile(overlayPath)
	if err != nil {
		t.Fatalf("failed to read %s: %v", overlayPath, err)
	}
	content := string(data)

	// Verify the file is valid JSON
	var parsed any
	if err := json.Unmarshal(data, &parsed); err != nil {
		t.Fatalf("vss-overlay.json is not valid JSON: %v", err)
	}

	// Check for required custom signals
	requiredSignals := []string{
		"Vehicle.Parking.SessionActive",
		"Vehicle.Command.Door.Lock",
		"Vehicle.Command.Door.Response",
	}

	for _, signal := range requiredSignals {
		t.Run(signal, func(t *testing.T) {
			if !strings.Contains(content, signal) {
				t.Errorf("vss-overlay.json should define signal %q", signal)
			}
		})
	}

	// Skeptic finding: check for duplicate JSON keys at the top level.
	// JSON spec says duplicate keys have undefined behavior; kuksa-databroker
	// may apply stricter parsing than Go's json.Unmarshal (which silently
	// accepts duplicates, last value wins).
	t.Run("no duplicate top-level keys", func(t *testing.T) {
		dec := json.NewDecoder(strings.NewReader(string(data)))
		tok, err := dec.Token()
		if err != nil {
			t.Fatalf("failed to read JSON token: %v", err)
		}
		if delim, ok := tok.(json.Delim); !ok || delim != '{' {
			t.Fatalf("expected JSON object, got %v", tok)
		}

		seen := make(map[string]int)
		depth := 1
		for dec.More() {
			tok, err := dec.Token()
			if err != nil {
				t.Fatalf("failed to read JSON token: %v", err)
			}
			// Track nesting depth to only count top-level keys
			switch v := tok.(type) {
			case json.Delim:
				switch v {
				case '{', '[':
					depth++
				case '}', ']':
					depth--
				}
			case string:
				if depth == 1 {
					seen[v]++
					if seen[v] > 1 {
						t.Errorf("duplicate top-level JSON key %q (appeared %d times)", v, seen[v])
					}
				}
			}
		}
	})
}
