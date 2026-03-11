package cmd

import (
	"fmt"
	"strings"
)

// parseFlag extracts the value of a --key=value flag from args.
// Returns the value and true if found, or empty string and false if not.
func parseFlag(args []string, name string) (string, bool) {
	prefix := "--" + name + "="
	for _, arg := range args {
		if strings.HasPrefix(arg, prefix) {
			return strings.TrimPrefix(arg, prefix), true
		}
	}
	return "", false
}

// requireFlag extracts a required --key=value flag from args.
// Returns the value if found, or an error mentioning the flag name.
func requireFlag(args []string, name string) (string, error) {
	val, ok := parseFlag(args, name)
	if !ok || val == "" {
		return "", fmt.Errorf("required flag --%s is missing", name)
	}
	return val, nil
}
