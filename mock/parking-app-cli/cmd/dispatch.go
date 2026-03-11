package cmd

import "fmt"

// ValidSubcommands lists all recognized subcommands.
var ValidSubcommands = []string{
	"lookup", "adapter-info",
	"install", "watch", "list", "remove", "status",
	"start-session", "stop-session",
}

// Dispatch routes the given subcommand name to its handler.
// Returns an error if the subcommand is unknown or if the handler fails.
func Dispatch(subcmd string, args []string) error {
	// TODO: implement proper dispatch
	switch subcmd {
	case "lookup", "adapter-info", "install", "watch", "list",
		"remove", "status", "start-session", "stop-session":
		return fmt.Errorf("subcommand %q: not yet implemented", subcmd)
	default:
		return fmt.Errorf("unknown subcommand: %q", subcmd)
	}
}

// IsValidSubcommand checks whether the given name is a known subcommand.
func IsValidSubcommand(name string) bool {
	for _, s := range ValidSubcommands {
		if s == name {
			return true
		}
	}
	return false
}
