// PARKING_CLI simulates the Kotlin PARKING_APP for parking session management.
package main

import (
	"bufio"
	"flag"
	"fmt"
	"os"
	"strings"

	"github.com/sdv-parking-demo/backend/parking-cli/internal/config"
	"github.com/sdv-parking-demo/backend/parking-cli/internal/repl"
)

// Exit codes
const (
	ExitSuccess         = 0
	ExitCommandFailed   = 1
	ExitInvalidArgs     = 2
	ExitConnectionError = 3
)

func main() {
	// Parse flags
	commandFlag := flag.String("c", "", "Execute single command and exit")
	commandFlagLong := flag.String("command", "", "Execute single command and exit")
	jsonFlag := flag.Bool("json", false, "Output in JSON format")
	quietFlag := flag.Bool("q", false, "Suppress informational messages")
	quietFlagLong := flag.Bool("quiet", false, "Suppress informational messages")
	flag.Parse()

	// Load configuration
	cfg := config.Load()

	// Create REPL
	r := repl.New(cfg)
	r.SetJSONOutput(*jsonFlag)
	r.SetQuiet(*quietFlag || *quietFlagLong)

	// Determine execution mode
	command := *commandFlag
	if command == "" {
		command = *commandFlagLong
	}

	// Check for positional arguments (e.g., parking-cli adapters)
	if command == "" && flag.NArg() > 0 {
		command = strings.Join(flag.Args(), " ")
	}

	// Check for piped input
	stdinInfo, _ := os.Stdin.Stat()
	isPiped := (stdinInfo.Mode() & os.ModeCharDevice) == 0

	// Non-interactive mode: execute single command
	if command != "" {
		r.SetNonInteractive(true)
		if err := r.Execute(command); err != nil {
			r.Close()
			exitCode := determineExitCode(err)
			os.Exit(exitCode)
		}
		r.Close()
		os.Exit(ExitSuccess)
	}

	// Piped input mode
	if isPiped {
		r.SetNonInteractive(true)
		scanner := bufio.NewScanner(os.Stdin)
		exitCode := ExitSuccess
		for scanner.Scan() {
			line := strings.TrimSpace(scanner.Text())
			if line == "" {
				continue
			}
			if err := r.Execute(line); err != nil {
				exitCode = determineExitCode(err)
			}
		}
		if scanner.Err() != nil {
			fmt.Fprintf(os.Stderr, "Error reading stdin: %v\n", scanner.Err())
			r.Close()
			os.Exit(ExitInvalidArgs)
		}
		r.Close()
		os.Exit(exitCode)
	}

	// Interactive mode
	if err := r.Run(); err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(ExitCommandFailed)
	}
}

// determineExitCode maps an error to an exit code.
func determineExitCode(err error) int {
	errStr := err.Error()
	if strings.Contains(errStr, "cannot connect") ||
		strings.Contains(errStr, "connection refused") ||
		strings.Contains(errStr, "timeout") ||
		strings.Contains(errStr, "failed to connect") {
		return ExitConnectionError
	}
	if strings.Contains(errStr, "missing") ||
		strings.Contains(errStr, "invalid") ||
		strings.Contains(errStr, "unknown command") ||
		strings.Contains(errStr, "Usage:") {
		return ExitInvalidArgs
	}
	return ExitCommandFailed
}
