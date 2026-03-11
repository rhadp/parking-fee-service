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
// serviceURL is the PARKING_FEE_SERVICE URL, updateAddr is the UPDATE_SERVICE gRPC address,
// and adaptorAddr is the PARKING_OPERATOR_ADAPTOR gRPC address.
func Dispatch(subcmd string, args []string, serviceURL, updateAddr, adaptorAddr string) error {
	switch subcmd {
	case "lookup":
		return RunLookup(args, serviceURL)
	case "adapter-info":
		return RunAdapterInfo(args, serviceURL)
	case "install":
		return RunInstall(args, updateAddr)
	case "watch":
		return RunWatch(args, updateAddr)
	case "list":
		return RunList(args, updateAddr)
	case "remove":
		return RunRemove(args, updateAddr)
	case "status":
		return RunStatus(args, updateAddr)
	case "start-session":
		return RunStartSession(args, adaptorAddr)
	case "stop-session":
		return RunStopSession(args, adaptorAddr)
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
