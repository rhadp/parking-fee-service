package cmd

import "fmt"

// subcommands maps subcommand names to their handler functions.
var subcommands = map[string]func(args []string) error{
	"lookup":        runLookup,
	"adapter-info":  runAdapterInfo,
	"install":       runInstall,
	"watch":         runWatch,
	"list":          runList,
	"remove":        runRemove,
	"status":        runStatus,
	"start-session": runStartSession,
	"stop-session":  runStopSession,
}

// SubcommandNames returns all subcommand names in display order.
func SubcommandNames() []string {
	return []string{
		"lookup",
		"adapter-info",
		"install",
		"watch",
		"list",
		"remove",
		"status",
		"start-session",
		"stop-session",
	}
}

// Run dispatches to the appropriate subcommand handler.
func Run(name string, args []string) error {
	fn, ok := subcommands[name]
	if !ok {
		return fmt.Errorf("unknown subcommand '%s'\nAvailable subcommands: %v", name, SubcommandNames())
	}
	return fn(args)
}
