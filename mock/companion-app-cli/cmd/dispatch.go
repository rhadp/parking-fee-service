package cmd

import "fmt"

// subcommands maps subcommand names to their handler functions.
var subcommands = map[string]func(args []string) error{
	"lock":   runLock,
	"unlock": runUnlock,
	"status": runStatus,
}

// SubcommandNames returns all subcommand names in display order.
func SubcommandNames() []string {
	return []string{"lock", "unlock", "status"}
}

// Run dispatches to the appropriate subcommand handler.
func Run(name string, args []string) error {
	fn, ok := subcommands[name]
	if !ok {
		return fmt.Errorf("unknown subcommand '%s'\nAvailable subcommands: %v", name, SubcommandNames())
	}
	return fn(args)
}
