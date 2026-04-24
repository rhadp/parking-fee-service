package databroker_test

import (
	"bytes"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"slices"
	"testing"
)

// vssNode represents a node in the VSS JSON overlay tree structure.
type vssNode struct {
	Type        string              `json:"type"`
	DataType    string              `json:"datatype"`
	Description string              `json:"description"`
	Children    map[string]*vssNode `json:"children"`
}

// readOverlayTree reads and parses the VSS overlay JSON file, returning
// the top-level map of nodes.
func readOverlayTree(t *testing.T) map[string]*vssNode {
	t.Helper()
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "vss-overlay.json"))
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}

	var tree map[string]*vssNode
	if err := json.Unmarshal(data, &tree); err != nil {
		t.Fatalf("failed to parse vss-overlay.json: %v", err)
	}
	return tree
}

// navigatePath walks the VSS tree from the root to the given dot-separated
// path segments and returns the node at the final position.
// Returns nil if any segment is missing.
func navigatePath(tree map[string]*vssNode, path []string) *vssNode {
	if len(path) == 0 {
		return nil
	}

	node, ok := tree[path[0]]
	if !ok || node == nil {
		return nil
	}

	for _, seg := range path[1:] {
		if node.Children == nil {
			return nil
		}
		node, ok = node.Children[seg]
		if !ok || node == nil {
			return nil
		}
	}
	return node
}

// TestVSSOverlayFormat validates the structure and content of the VSS overlay
// JSON file. This is a static test that reads the file directly — it does not
// require a running DATA_BROKER container.
//
// Test Spec: TS-02-5 (static validation), TS-02-P1 (completeness)
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3, 02-REQ-6.4
func TestVSSOverlayFormat(t *testing.T) {
	tree := readOverlayTree(t)

	t.Run("valid_json", func(t *testing.T) {
		if len(tree) == 0 {
			t.Fatal("overlay JSON is empty")
		}
	})

	// Verify Vehicle root branch exists.
	t.Run("Vehicle_root_branch", func(t *testing.T) {
		vehicle, ok := tree["Vehicle"]
		if !ok {
			t.Fatal("Vehicle root node not found in overlay")
		}
		if vehicle.Type != "branch" {
			t.Errorf("Vehicle node type: got %q, want %q", vehicle.Type, "branch")
		}
		if vehicle.Children == nil {
			t.Fatal("Vehicle node has no children")
		}
	})

	// Verify branch nodes exist with correct types.
	branchPaths := []struct {
		name string
		path []string
	}{
		{"Vehicle.Parking", []string{"Vehicle", "Parking"}},
		{"Vehicle.Command", []string{"Vehicle", "Command"}},
		{"Vehicle.Command.Door", []string{"Vehicle", "Command", "Door"}},
	}
	for _, bp := range branchPaths {
		t.Run("branch/"+bp.name, func(t *testing.T) {
			node := navigatePath(tree, bp.path)
			if node == nil {
				t.Fatalf("branch node %s not found", bp.name)
			}
			if node.Type != "branch" {
				t.Errorf("branch %s type: got %q, want %q", bp.name, node.Type, "branch")
			}
			if node.Description == "" {
				t.Errorf("branch %s has empty description", bp.name)
			}
			if node.Children == nil {
				t.Errorf("branch %s has no children", bp.name)
			}
		})
	}

	// Verify custom signal leaf nodes with correct data types and node types.
	signalTests := []struct {
		name       string
		path       []string
		dataType   string
		validTypes []string // acceptable VSS node types (sensor, actuator, attribute)
	}{
		{
			name:       "Vehicle.Parking.SessionActive",
			path:       []string{"Vehicle", "Parking", "SessionActive"},
			dataType:   "boolean",
			validTypes: []string{"sensor", "actuator", "attribute"},
		},
		{
			name:       "Vehicle.Command.Door.Lock",
			path:       []string{"Vehicle", "Command", "Door", "Lock"},
			dataType:   "string",
			validTypes: []string{"sensor", "actuator", "attribute"},
		},
		{
			name:       "Vehicle.Command.Door.Response",
			path:       []string{"Vehicle", "Command", "Door", "Response"},
			dataType:   "string",
			validTypes: []string{"sensor", "actuator", "attribute"},
		},
	}

	for _, st := range signalTests {
		t.Run("signal/"+st.name, func(t *testing.T) {
			node := navigatePath(tree, st.path)
			if node == nil {
				t.Fatalf("signal %s not found in overlay", st.name)
			}

			t.Run("datatype", func(t *testing.T) {
				if node.DataType != st.dataType {
					t.Errorf("signal %s datatype: got %q, want %q", st.name, node.DataType, st.dataType)
				}
			})

			t.Run("leaf_type", func(t *testing.T) {
				if !slices.Contains(st.validTypes, node.Type) {
					t.Errorf("signal %s type: got %q, want one of %v", st.name, node.Type, st.validTypes)
				}
			})

			t.Run("not_branch", func(t *testing.T) {
				if node.Type == "branch" {
					t.Errorf("signal %s should be a leaf node, not a branch", st.name)
				}
			})

			t.Run("description", func(t *testing.T) {
				if node.Description == "" {
					t.Errorf("signal %s has empty description", st.name)
				}
			})
		})
	}

	// Verify the overlay defines exactly the expected children under Vehicle.
	t.Run("no_unexpected_entries", func(t *testing.T) {
		vehicle := tree["Vehicle"]
		if vehicle == nil {
			t.Skip("Vehicle root not found")
		}
		// The overlay should define exactly 2 top-level children under Vehicle:
		// Parking and Command.
		if len(vehicle.Children) != 2 {
			names := make([]string, 0, len(vehicle.Children))
			for k := range vehicle.Children {
				names = append(names, k)
			}
			t.Errorf("expected 2 children under Vehicle (Parking, Command), got %d: %v",
				len(vehicle.Children), names)
		}
	})
}

// TestVSSOverlayNoDuplicateKeys verifies the overlay JSON file has no
// duplicate keys at any nesting level by tokenizing the raw JSON. Go's
// encoding/json silently uses the last value for duplicate keys, which
// can hide defects (e.g., a signal defined as both "actuator" and "sensor"
// where the second definition silently wins).
//
// Addresses critical review finding about duplicate JSON keys.
//
// Requirements: 02-REQ-6.1, 02-REQ-6.2, 02-REQ-6.3
func TestVSSOverlayNoDuplicateKeys(t *testing.T) {
	root := repoRoot(t)
	data, err := os.ReadFile(filepath.Join(root, "deployments", "vss-overlay.json"))
	if err != nil {
		t.Fatalf("failed to read vss-overlay.json: %v", err)
	}

	dec := json.NewDecoder(bytes.NewReader(data))
	if err := checkDuplicatesInDecoder(t, dec, ""); err != nil {
		t.Fatalf("error while checking for duplicate keys: %v", err)
	}
}

// checkDuplicatesInDecoder walks a JSON token stream and fails the test
// if any object contains duplicate keys. path tracks the current location
// for error messages.
func checkDuplicatesInDecoder(t *testing.T, dec *json.Decoder, path string) error {
	t.Helper()

	tok, err := dec.Token()
	if err != nil {
		return err
	}

	switch v := tok.(type) {
	case json.Delim:
		switch v {
		case '{':
			keys := make(map[string]bool)
			for dec.More() {
				keyTok, err := dec.Token()
				if err != nil {
					return err
				}
				key, ok := keyTok.(string)
				if !ok {
					continue
				}

				fullPath := key
				if path != "" {
					fullPath = path + "." + key
				}

				if keys[key] {
					t.Errorf("duplicate JSON key %q at path %q", key, path)
				}
				keys[key] = true

				// Recursively check the value.
				if err := checkDuplicatesInDecoder(t, dec, fullPath); err != nil {
					return err
				}
			}
			// Consume closing '}'.
			if _, err := dec.Token(); err != nil {
				return err
			}
		case '[':
			idx := 0
			for dec.More() {
				elemPath := fmt.Sprintf("%s[%d]", path, idx)
				if err := checkDuplicatesInDecoder(t, dec, elemPath); err != nil {
					return err
				}
				idx++
			}
			// Consume closing ']'.
			if _, err := dec.Token(); err != nil {
				return err
			}
		}
	default:
		// Scalar value — nothing to check.
	}

	return nil
}
