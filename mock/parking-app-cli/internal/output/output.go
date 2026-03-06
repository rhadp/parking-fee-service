package output

import (
	"encoding/json"
	"fmt"
	"os"
)

// PrintJSON marshals v as indented JSON and prints to stdout.
func PrintJSON(v interface{}) error {
	data, err := json.MarshalIndent(v, "", "  ")
	if err != nil {
		return fmt.Errorf("failed to marshal JSON: %w", err)
	}
	fmt.Fprintln(os.Stdout, string(data))
	return nil
}

// PrintRawJSON prints raw JSON bytes with indentation to stdout.
func PrintRawJSON(raw []byte) error {
	var v interface{}
	if err := json.Unmarshal(raw, &v); err != nil {
		// If not valid JSON, print as-is
		fmt.Fprintln(os.Stdout, string(raw))
		return nil
	}
	return PrintJSON(v)
}
