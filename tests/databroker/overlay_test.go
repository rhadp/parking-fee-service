package databroker_test

import (
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// readOverlay reads the VSS overlay JSON file for static validation.
func readOverlay(t *testing.T) []byte {
	t.Helper()
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "vss-overlay.json"))
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}
	return data
}

// navigateToLeaf walks the overlay JSON tree and returns the leaf node at the
// given path. The last segment is the leaf (no children expected).
func navigateToLeaf(tree map[string]any, path string) map[string]any {
	parts := strings.Split(path, ".")
	if len(parts) == 0 {
		return nil
	}
	current := tree
	for i, part := range parts {
		node, ok := current[part]
		if !ok {
			return nil
		}
		nodeMap, ok := node.(map[string]any)
		if !ok {
			return nil
		}
		// Last segment: return the leaf node.
		if i == len(parts)-1 {
			return nodeMap
		}
		// Not the last segment: descend into children.
		children, hasChildren := nodeMap["children"]
		if !hasChildren {
			return nil
		}
		childMap, ok := children.(map[string]any)
		if !ok {
			return nil
		}
		current = childMap
	}
	return nil
}

// TestVSSOverlayFormat validates the VSS overlay JSON structure, verifying that all
// 3 custom signals are present with correct types, branch nodes are defined, and the
// JSON is structurally valid.
// Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestVSSOverlayFormat(t *testing.T) {
	data := readOverlay(t)

	// Parse the overlay as generic JSON.
	var tree map[string]any
	if err := json.Unmarshal(data, &tree); err != nil {
		t.Fatalf("overlay JSON is invalid: %v", err)
	}

	// Verify branch nodes exist and have correct types.
	type branchSpec struct {
		path string
	}
	branches := []branchSpec{
		{"Vehicle"},
		{"Vehicle.Parking"},
		{"Vehicle.Command"},
		{"Vehicle.Command.Door"},
	}
	for _, b := range branches {
		t.Run("branch/"+b.path, func(t *testing.T) {
			node := navigateToLeaf(tree, b.path)
			if node == nil {
				t.Fatalf("branch node %s not found in overlay", b.path)
			}

			t.Run("type", func(t *testing.T) {
				nodeType, ok := node["type"].(string)
				if !ok {
					t.Fatalf("branch node %s missing 'type' field", b.path)
				}
				if nodeType != "branch" {
					t.Errorf("branch node %s: expected type 'branch', got %q", b.path, nodeType)
				}
			})

			t.Run("children", func(t *testing.T) {
				_, hasChildren := node["children"]
				if !hasChildren {
					t.Errorf("branch node %s missing 'children' field", b.path)
				}
			})
		})
	}

	// Verify leaf signals exist with correct properties.
	type leafSpec struct {
		path       string
		datatype   string
		leafType   string // "sensor", "actuator", or "attribute"
		notBranch  bool   // must NOT be a branch
	}
	leaves := []leafSpec{
		{
			path:      "Vehicle.Parking.SessionActive",
			datatype:  "boolean",
			leafType:  "sensor",
			notBranch: true,
		},
		{
			path:      "Vehicle.Command.Door.Lock",
			datatype:  "string",
			leafType:  "actuator",
			notBranch: true,
		},
		{
			path:      "Vehicle.Command.Door.Response",
			datatype:  "string",
			leafType:  "sensor",
			notBranch: true,
		},
	}
	for _, leaf := range leaves {
		t.Run("leaf/"+leaf.path, func(t *testing.T) {
			node := navigateToLeaf(tree, leaf.path)
			if node == nil {
				t.Fatalf("leaf signal %s not found in overlay", leaf.path)
			}

			t.Run("datatype", func(t *testing.T) {
				dt, ok := node["datatype"].(string)
				if !ok {
					t.Fatalf("signal %s missing 'datatype' field", leaf.path)
				}
				if dt != leaf.datatype {
					t.Errorf("signal %s: expected datatype %q, got %q", leaf.path, leaf.datatype, dt)
				}
			})

			t.Run("leaf_type", func(t *testing.T) {
				lt, ok := node["type"].(string)
				if !ok {
					t.Fatalf("signal %s missing 'type' field", leaf.path)
				}
				if lt != leaf.leafType {
					t.Errorf("signal %s: expected leaf type %q, got %q", leaf.path, leaf.leafType, lt)
				}
			})

			t.Run("not_branch", func(t *testing.T) {
				lt, _ := node["type"].(string)
				if lt == "branch" {
					t.Errorf("signal %s should be a leaf signal, not a branch", leaf.path)
				}
			})

			t.Run("description", func(t *testing.T) {
				desc, ok := node["description"].(string)
				if !ok || desc == "" {
					t.Errorf("signal %s missing or empty 'description' field", leaf.path)
				}
			})
		})
	}
}

// TestVSSOverlayNoDuplicateKeys verifies that the overlay JSON file contains no
// duplicate keys at any nesting depth. Duplicate JSON keys are technically allowed
// by the JSON spec but would silently overwrite earlier values, potentially losing
// signal definitions.
// Requirement: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestVSSOverlayNoDuplicateKeys(t *testing.T) {
	data := readOverlay(t)

	// Use json.Decoder with custom logic to detect duplicate keys.
	duplicates := findDuplicateKeys(data)
	for _, dup := range duplicates {
		t.Errorf("duplicate JSON key found: %s", dup)
	}
}

// findDuplicateKeys scans raw JSON bytes for duplicate keys at any nesting level.
// Returns a list of human-readable descriptions for each duplicate found.
func findDuplicateKeys(data []byte) []string {
	var duplicates []string
	checkDuplicates(data, "", &duplicates)
	return duplicates
}

// checkDuplicates recursively parses JSON objects to detect duplicate keys.
func checkDuplicates(data []byte, prefix string, duplicates *[]string) {
	dec := json.NewDecoder(strings.NewReader(string(data)))

	// Read opening token.
	tok, err := dec.Token()
	if err != nil {
		return
	}

	delim, ok := tok.(json.Delim)
	if !ok || delim != '{' {
		return // Not an object, nothing to check.
	}

	seen := make(map[string]bool)
	for dec.More() {
		// Read key.
		keyTok, keyErr := dec.Token()
		if keyErr != nil {
			return
		}
		key, ok := keyTok.(string)
		if !ok {
			return
		}

		fullKey := key
		if prefix != "" {
			fullKey = prefix + "." + key
		}

		if seen[key] {
			*duplicates = append(*duplicates, fullKey)
		}
		seen[key] = true

		// Read value — capture raw JSON for recursive check.
		var rawValue json.RawMessage
		if valErr := dec.Decode(&rawValue); valErr != nil {
			return
		}

		// Recursively check nested objects.
		checkDuplicates(rawValue, fullKey, duplicates)
	}
}
