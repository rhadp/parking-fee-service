// Package output provides JSON formatting and error display helpers.
package output

import (
	"encoding/json"
	"fmt"
	"os"
)

// PrintJSON formats and prints the given data as indented JSON to stdout.
func PrintJSON(data any) error {
	enc := json.NewEncoder(os.Stdout)
	enc.SetIndent("", "  ")
	return enc.Encode(data)
}

// PrintRawJSON formats and prints raw JSON bytes as indented JSON to stdout.
func PrintRawJSON(raw []byte) error {
	var v any
	if err := json.Unmarshal(raw, &v); err != nil {
		// If not valid JSON, print as-is
		fmt.Println(string(raw))
		return nil
	}
	return PrintJSON(v)
}

// PrintError prints an error message to stderr.
func PrintError(format string, args ...any) {
	fmt.Fprintf(os.Stderr, "Error: "+format+"\n", args...)
}
