package cmd

import "fmt"

// ValidSubcommands lists all recognized subcommands.
var ValidSubcommands = []string{"lock", "unlock", "status"}

// Dispatch routes the given subcommand name to its handler.
func Dispatch(subcmd string, args []string, gatewayURL, bearerToken string) error {
	switch subcmd {
	case "lock":
		return RunLock(args, gatewayURL, bearerToken)
	case "unlock":
		return RunUnlock(args, gatewayURL, bearerToken)
	case "status":
		return RunStatus(args, gatewayURL, bearerToken)
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
