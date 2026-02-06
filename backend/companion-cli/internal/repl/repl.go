// Package repl provides the interactive command interface for COMPANION_CLI.
package repl

import (
	"bufio"
	"context"
	"encoding/json"
	"fmt"
	"io"
	"os"
	"strings"

	"github.com/sdv-parking-demo/backend/companion-cli/internal/client"
	"github.com/sdv-parking-demo/backend/companion-cli/internal/config"
)

// Command represents a CLI command.
type Command struct {
	Name        string
	Description string
	Usage       string
	Handler     func(args []string) error
}

// JSONOutput wraps command results for JSON output mode.
type JSONOutput struct {
	Success bool        `json:"success"`
	Command string      `json:"command"`
	Result  interface{} `json:"result,omitempty"`
	Error   *JSONError  `json:"error,omitempty"`
}

// JSONError represents an error in JSON output.
type JSONError struct {
	Code    string `json:"code"`
	Message string `json:"message"`
	Details string `json:"details,omitempty"`
}

// REPL provides the interactive command interface.
type REPL struct {
	prompt       string
	commands     map[string]Command
	running      bool
	gateway      *client.GatewayClient
	cfg          *config.Config
	jsonOutput   bool
	quiet        bool
	input        io.Reader
	output       io.Writer
	lastError    error
	nonInteractive bool
}

// New creates a new REPL with the given configuration.
func New(cfg *config.Config, gateway *client.GatewayClient) *REPL {
	r := &REPL{
		prompt:   "> ",
		commands: make(map[string]Command),
		running:  true,
		gateway:  gateway,
		cfg:      cfg,
		input:    os.Stdin,
		output:   os.Stdout,
	}

	r.registerCommands()
	return r
}

// SetJSONOutput enables/disables JSON output mode.
func (r *REPL) SetJSONOutput(enabled bool) {
	r.jsonOutput = enabled
}

// SetQuiet enables/disables quiet mode.
func (r *REPL) SetQuiet(enabled bool) {
	r.quiet = enabled
}

// SetNonInteractive sets non-interactive mode.
func (r *REPL) SetNonInteractive(enabled bool) {
	r.nonInteractive = enabled
}

// SetInput sets the input reader.
func (r *REPL) SetInput(input io.Reader) {
	r.input = input
}

// SetOutput sets the output writer.
func (r *REPL) SetOutput(output io.Writer) {
	r.output = output
}

// GetLastError returns the last error encountered.
func (r *REPL) GetLastError() error {
	return r.lastError
}

// registerCommands registers all available commands.
func (r *REPL) registerCommands() {
	r.commands["lock"] = Command{
		Name:        "lock",
		Description: "Send a lock command to the vehicle",
		Usage:       "lock",
		Handler:     r.handleLock,
	}

	r.commands["unlock"] = Command{
		Name:        "unlock",
		Description: "Send an unlock command to the vehicle",
		Usage:       "unlock",
		Handler:     r.handleUnlock,
	}

	r.commands["status"] = Command{
		Name:        "status",
		Description: "Check the status of a command",
		Usage:       "status <command_id>",
		Handler:     r.handleStatus,
	}

	r.commands["ping"] = Command{
		Name:        "ping",
		Description: "Test connectivity to the cloud gateway",
		Usage:       "ping",
		Handler:     r.handlePing,
	}

	r.commands["help"] = Command{
		Name:        "help",
		Description: "Display available commands",
		Usage:       "help",
		Handler:     r.handleHelp,
	}

	r.commands["quit"] = Command{
		Name:        "quit",
		Description: "Exit the CLI",
		Usage:       "quit",
		Handler:     r.handleQuit,
	}

	r.commands["exit"] = Command{
		Name:        "exit",
		Description: "Exit the CLI",
		Usage:       "exit",
		Handler:     r.handleQuit,
	}
}

// Run starts the REPL loop.
func (r *REPL) Run() error {
	if !r.quiet && !r.nonInteractive {
		r.printWelcome()
	}

	scanner := bufio.NewScanner(r.input)
	for r.running {
		if !r.nonInteractive {
			fmt.Fprint(r.output, r.prompt)
		}

		if !scanner.Scan() {
			break
		}

		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}

		if err := r.Execute(line); err != nil {
			r.lastError = err
			if r.nonInteractive {
				return err
			}
		}
	}

	if err := scanner.Err(); err != nil {
		return fmt.Errorf("error reading input: %w", err)
	}

	return nil
}

// Execute executes a single command line.
func (r *REPL) Execute(line string) error {
	parts := strings.Fields(line)
	if len(parts) == 0 {
		return nil
	}

	cmdName := strings.ToLower(parts[0])
	args := parts[1:]

	cmd, exists := r.commands[cmdName]
	if !exists {
		msg := "Unknown command. Type 'help' for available commands."
		if r.jsonOutput {
			r.outputJSON("unknown", nil, &JSONError{Code: "UNKNOWN_COMMAND", Message: msg})
		} else {
			fmt.Fprintln(r.output, msg)
		}
		return fmt.Errorf("unknown command: %s", cmdName)
	}

	err := cmd.Handler(args)
	if err != nil && !r.jsonOutput {
		fmt.Fprintf(r.output, "Error: %v\n", err)
	}
	return err
}

// printWelcome prints the welcome message.
func (r *REPL) printWelcome() {
	fmt.Fprintln(r.output, "COMPANION_CLI - Remote Vehicle Control Simulator")
	fmt.Fprintln(r.output, "")
	fmt.Fprintf(r.output, "Connected to: %s\n", r.cfg.CloudGatewayURL)
	fmt.Fprintf(r.output, "VIN: %s\n", r.cfg.VIN)
	fmt.Fprintln(r.output, "")
	fmt.Fprintln(r.output, "Type 'help' for available commands.")
	fmt.Fprintln(r.output, "")
}

// handleLock handles the lock command.
func (r *REPL) handleLock(args []string) error {
	ctx := context.Background()
	resp, err := r.gateway.SendLockCommand(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("lock", nil, &JSONError{Code: "COMMAND_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("lock", resp, nil)
	} else {
		fmt.Fprintf(r.output, "Lock command sent successfully\n")
		fmt.Fprintf(r.output, "  Command ID: %s\n", resp.CommandID)
		fmt.Fprintf(r.output, "  Status: %s\n", resp.Status)
	}
	return nil
}

// handleUnlock handles the unlock command.
func (r *REPL) handleUnlock(args []string) error {
	ctx := context.Background()
	resp, err := r.gateway.SendUnlockCommand(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("unlock", nil, &JSONError{Code: "COMMAND_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("unlock", resp, nil)
	} else {
		fmt.Fprintf(r.output, "Unlock command sent successfully\n")
		fmt.Fprintf(r.output, "  Command ID: %s\n", resp.CommandID)
		fmt.Fprintf(r.output, "  Status: %s\n", resp.Status)
	}
	return nil
}

// handleStatus handles the status command.
func (r *REPL) handleStatus(args []string) error {
	if len(args) < 1 {
		msg := "Usage: status <command_id>"
		if r.jsonOutput {
			r.outputJSON("status", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		} else {
			fmt.Fprintln(r.output, msg)
		}
		return fmt.Errorf("missing command_id argument")
	}

	commandID := args[0]
	ctx := context.Background()
	resp, err := r.gateway.GetCommandStatus(ctx, commandID)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("status", nil, &JSONError{Code: "STATUS_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("status", resp, nil)
	} else {
		fmt.Fprintf(r.output, "Command Status:\n")
		fmt.Fprintf(r.output, "  Command ID: %s\n", resp.CommandID)
		fmt.Fprintf(r.output, "  Type: %s\n", resp.CommandType)
		fmt.Fprintf(r.output, "  Status: %s\n", resp.Status)
		fmt.Fprintf(r.output, "  Created At: %s\n", resp.CreatedAt)
		if resp.CompletedAt != nil {
			fmt.Fprintf(r.output, "  Completed At: %s\n", *resp.CompletedAt)
		}
		if resp.Status == "success" {
			fmt.Fprintln(r.output, "  -> Command completed successfully")
		} else if resp.ErrorMessage != "" {
			fmt.Fprintf(r.output, "  Error: %s - %s\n", resp.ErrorCode, resp.ErrorMessage)
		}
	}
	return nil
}

// handlePing handles the ping command.
func (r *REPL) handlePing(args []string) error {
	ctx := context.Background()
	err := r.gateway.Ping(ctx)

	result := map[string]interface{}{
		"service":   "CLOUD_GATEWAY",
		"address":   r.cfg.CloudGatewayURL,
		"connected": err == nil,
	}

	if err != nil {
		result["error"] = err.Error()
		if r.jsonOutput {
			r.outputJSON("ping", result, nil)
		} else {
			fmt.Fprintf(r.output, "CLOUD_GATEWAY: OFFLINE (%s)\n", err)
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("ping", result, nil)
	} else {
		fmt.Fprintf(r.output, "CLOUD_GATEWAY: OK (%s)\n", r.cfg.CloudGatewayURL)
	}
	return nil
}

// handleHelp handles the help command.
func (r *REPL) handleHelp(args []string) error {
	if r.jsonOutput {
		commands := make([]map[string]string, 0, len(r.commands))
		for _, cmd := range r.commands {
			if cmd.Name == "exit" {
				continue // Skip alias
			}
			commands = append(commands, map[string]string{
				"name":        cmd.Name,
				"description": cmd.Description,
				"usage":       cmd.Usage,
			})
		}
		r.outputJSON("help", map[string]interface{}{"commands": commands}, nil)
	} else {
		fmt.Fprintln(r.output, "Available commands:")
		fmt.Fprintln(r.output, "")
		fmt.Fprintln(r.output, "  lock       Send a lock command to the vehicle")
		fmt.Fprintln(r.output, "  unlock     Send an unlock command to the vehicle")
		fmt.Fprintln(r.output, "  status     Check the status of a command (usage: status <command_id>)")
		fmt.Fprintln(r.output, "  ping       Test connectivity to the cloud gateway")
		fmt.Fprintln(r.output, "  help       Display this help message")
		fmt.Fprintln(r.output, "  quit       Exit the CLI")
	}
	return nil
}

// handleQuit handles the quit command.
func (r *REPL) handleQuit(args []string) error {
	r.running = false
	if !r.quiet && !r.jsonOutput {
		fmt.Fprintln(r.output, "Goodbye!")
	}
	return nil
}

// outputJSON outputs a JSON response.
func (r *REPL) outputJSON(command string, result interface{}, err *JSONError) {
	output := JSONOutput{
		Success: err == nil,
		Command: command,
		Result:  result,
		Error:   err,
	}
	data, _ := json.MarshalIndent(output, "", "  ")
	fmt.Fprintln(r.output, string(data))
}

// GetCommands returns all registered commands.
func (r *REPL) GetCommands() map[string]Command {
	return r.commands
}
