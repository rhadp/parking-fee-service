// Package repl provides the interactive command interface for PARKING_CLI.
package repl

import (
	"bufio"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"os"
	"strconv"
	"strings"
	"time"

	"github.com/sdv-parking-demo/backend/parking-cli/internal/client"
	"github.com/sdv-parking-demo/backend/parking-cli/internal/config"
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

// PingResult represents connectivity test result for a service.
type PingResult struct {
	Service   string `json:"service"`
	Address   string `json:"address"`
	Connected bool   `json:"connected"`
	Error     string `json:"error,omitempty"`
}

// Clients holds all gRPC/HTTP clients.
type Clients struct {
	DataBroker    *client.DataBrokerClient
	ParkingFee    *client.ParkingFeeClient
	UpdateService *client.UpdateServiceClient
	Adaptor       *client.ParkingAdaptorClient
	Locking       *client.LockingServiceClient
}

// REPL provides the interactive command interface.
type REPL struct {
	prompt         string
	commands       map[string]Command
	running        bool
	cfg            *config.Config
	clients        *Clients
	jsonOutput     bool
	quiet          bool
	input          io.Reader
	output         io.Writer
	lastError      error
	nonInteractive bool
}

// New creates a new REPL with the given configuration.
func New(cfg *config.Config) *REPL {
	r := &REPL{
		prompt:   "> ",
		commands: make(map[string]Command),
		running:  true,
		cfg:      cfg,
		clients:  &Clients{},
		input:    os.Stdin,
		output:   os.Stdout,
	}

	// Initialize HTTP clients (always available)
	r.clients.ParkingFee = client.NewParkingFeeClient(cfg.ParkingFeeServiceURL, cfg.Timeout)

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

// Close closes all client connections.
func (r *REPL) Close() {
	if r.clients.DataBroker != nil {
		r.clients.DataBroker.Close()
	}
	if r.clients.UpdateService != nil {
		r.clients.UpdateService.Close()
	}
	if r.clients.Adaptor != nil {
		r.clients.Adaptor.Close()
	}
	if r.clients.Locking != nil {
		r.clients.Locking.Close()
	}
}

// registerCommands registers all available commands.
func (r *REPL) registerCommands() {
	r.commands["location"] = Command{
		Name:        "location",
		Description: "Get or set vehicle location",
		Usage:       "location [<lat> <lng>]",
		Handler:     r.handleLocation,
	}

	r.commands["zone"] = Command{
		Name:        "zone",
		Description: "Look up parking zone at current location",
		Usage:       "zone",
		Handler:     r.handleZone,
	}

	r.commands["adapters"] = Command{
		Name:        "adapters",
		Description: "List installed adapters",
		Usage:       "adapters",
		Handler:     r.handleAdapters,
	}

	r.commands["install"] = Command{
		Name:        "install",
		Description: "Install an adapter from registry",
		Usage:       "install <image_ref>",
		Handler:     r.handleInstall,
	}

	r.commands["uninstall"] = Command{
		Name:        "uninstall",
		Description: "Uninstall an adapter",
		Usage:       "uninstall <adapter_id>",
		Handler:     r.handleUninstall,
	}

	r.commands["start"] = Command{
		Name:        "start",
		Description: "Start a parking session",
		Usage:       "start <zone_id>",
		Handler:     r.handleStart,
	}

	r.commands["stop"] = Command{
		Name:        "stop",
		Description: "Stop the current parking session",
		Usage:       "stop",
		Handler:     r.handleStop,
	}

	r.commands["session"] = Command{
		Name:        "session",
		Description: "Get current session status",
		Usage:       "session",
		Handler:     r.handleSession,
	}

	r.commands["locks"] = Command{
		Name:        "locks",
		Description: "Display door lock states",
		Usage:       "locks",
		Handler:     r.handleLocks,
	}

	r.commands["ping"] = Command{
		Name:        "ping",
		Description: "Test connectivity to all services",
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
	defer r.Close()

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
	fmt.Fprintln(r.output, "PARKING_CLI - Parking Session Management Simulator")
	fmt.Fprintln(r.output, "")
	fmt.Fprintln(r.output, "Service Endpoints:")
	fmt.Fprintf(r.output, "  DATA_BROKER:       %s\n", r.cfg.DataBrokerAddr)
	fmt.Fprintf(r.output, "  PARKING_FEE_SVC:   %s\n", r.cfg.ParkingFeeServiceURL)
	fmt.Fprintf(r.output, "  UPDATE_SERVICE:    %s\n", r.cfg.UpdateServiceAddr)
	fmt.Fprintf(r.output, "  PARKING_ADAPTOR:   %s\n", r.cfg.ParkingAdaptorAddr)
	fmt.Fprintf(r.output, "  LOCKING_SERVICE:   %s\n", r.cfg.LockingServiceAddr)
	fmt.Fprintln(r.output, "")
	fmt.Fprintln(r.output, "Type 'help' for available commands.")
	fmt.Fprintln(r.output, "")
}

// ensureDataBroker ensures the DATA_BROKER client is connected.
func (r *REPL) ensureDataBroker() error {
	if r.clients.DataBroker != nil {
		return nil
	}
	var err error
	r.clients.DataBroker, err = client.NewDataBrokerClient(r.cfg.DataBrokerAddr, r.cfg.Timeout)
	return err
}

// ensureUpdateService ensures the UPDATE_SERVICE client is connected.
func (r *REPL) ensureUpdateService() error {
	if r.clients.UpdateService != nil {
		return nil
	}
	var err error
	r.clients.UpdateService, err = client.NewUpdateServiceClient(r.cfg.UpdateServiceAddr, r.cfg.Timeout)
	return err
}

// ensureAdaptor ensures the PARKING_ADAPTOR client is connected.
func (r *REPL) ensureAdaptor() error {
	if r.clients.Adaptor != nil {
		return nil
	}
	var err error
	r.clients.Adaptor, err = client.NewParkingAdaptorClient(r.cfg.ParkingAdaptorAddr, r.cfg.Timeout)
	return err
}

// ensureLocking ensures the LOCKING_SERVICE client is connected.
func (r *REPL) ensureLocking() error {
	if r.clients.Locking != nil {
		return nil
	}
	var err error
	r.clients.Locking, err = client.NewLockingServiceClient(r.cfg.LockingServiceAddr, r.cfg.Timeout)
	return err
}

// handleLocation handles the location command.
func (r *REPL) handleLocation(args []string) error {
	if err := r.ensureDataBroker(); err != nil {
		if r.jsonOutput {
			r.outputJSON("location", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	// Get location if no args
	if len(args) == 0 {
		lat, lng, err := r.clients.DataBroker.GetLocation(ctx)
		if err != nil {
			if r.jsonOutput {
				r.outputJSON("location", nil, &JSONError{Code: "GET_FAILED", Message: err.Error()})
			}
			return err
		}

		result := map[string]float64{"latitude": lat, "longitude": lng}
		if r.jsonOutput {
			r.outputJSON("location", result, nil)
		} else {
			fmt.Fprintf(r.output, "Current location: (%f, %f)\n", lat, lng)
		}
		return nil
	}

	// Set location
	if len(args) < 2 {
		msg := "Usage: location <lat> <lng>"
		if r.jsonOutput {
			r.outputJSON("location", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		} else {
			fmt.Fprintln(r.output, msg)
		}
		return fmt.Errorf("missing arguments")
	}

	lat, err := strconv.ParseFloat(args[0], 64)
	if err != nil {
		msg := fmt.Sprintf("Invalid latitude: %s", args[0])
		if r.jsonOutput {
			r.outputJSON("location", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		}
		return errors.New(msg)
	}

	lng, err := strconv.ParseFloat(args[1], 64)
	if err != nil {
		msg := fmt.Sprintf("Invalid longitude: %s", args[1])
		if r.jsonOutput {
			r.outputJSON("location", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		}
		return errors.New(msg)
	}

	if err := r.clients.DataBroker.SetLocation(ctx, lat, lng); err != nil {
		if r.jsonOutput {
			r.outputJSON("location", nil, &JSONError{Code: "SET_FAILED", Message: err.Error()})
		}
		return err
	}

	result := map[string]float64{"latitude": lat, "longitude": lng}
	if r.jsonOutput {
		r.outputJSON("location", result, nil)
	} else {
		fmt.Fprintf(r.output, "Location set to (%f, %f)\n", lat, lng)
	}
	return nil
}

// handleZone handles the zone command.
func (r *REPL) handleZone(args []string) error {
	if err := r.ensureDataBroker(); err != nil {
		if r.jsonOutput {
			r.outputJSON("zone", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	lat, lng, err := r.clients.DataBroker.GetLocation(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("zone", nil, &JSONError{Code: "LOCATION_ERROR", Message: err.Error()})
		}
		return err
	}

	zone, err := r.clients.ParkingFee.GetZone(ctx, lat, lng)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("zone", nil, &JSONError{Code: "LOOKUP_FAILED", Message: err.Error()})
		}
		return err
	}

	if zone == nil {
		if r.jsonOutput {
			r.outputJSON("zone", map[string]bool{"found": false}, nil)
		} else {
			fmt.Fprintln(r.output, "No parking zone detected at current location")
		}
		return nil
	}

	if r.jsonOutput {
		r.outputJSON("zone", zone, nil)
	} else {
		fmt.Fprintln(r.output, "Parking Zone:")
		fmt.Fprintf(r.output, "  Zone ID: %s\n", zone.ZoneID)
		fmt.Fprintf(r.output, "  Operator: %s\n", zone.OperatorName)
		fmt.Fprintf(r.output, "  Rate: %.2f %s/hour\n", zone.HourlyRate, zone.Currency)
		if zone.AdapterImageRef != "" {
			fmt.Fprintf(r.output, "  Adapter: %s\n", zone.AdapterImageRef)
		}
	}
	return nil
}

// handleAdapters handles the adapters command.
func (r *REPL) handleAdapters(args []string) error {
	if err := r.ensureUpdateService(); err != nil {
		if r.jsonOutput {
			r.outputJSON("adapters", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	adapters, err := r.clients.UpdateService.ListAdapters(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("adapters", nil, &JSONError{Code: "LIST_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("adapters", map[string]interface{}{"adapters": adapters, "count": len(adapters)}, nil)
	} else {
		if len(adapters) == 0 {
			fmt.Fprintln(r.output, "No adapters installed")
			return nil
		}
		fmt.Fprintf(r.output, "Installed Adapters (%d):\n", len(adapters))
		for _, a := range adapters {
			fmt.Fprintf(r.output, "  %s\n", a.AdapterID)
			fmt.Fprintf(r.output, "    Image: %s\n", a.ImageRef)
			fmt.Fprintf(r.output, "    Version: %s\n", a.Version)
			fmt.Fprintf(r.output, "    State: %s\n", a.State)
			if a.ErrorMessage != "" {
				fmt.Fprintf(r.output, "    Error: %s\n", a.ErrorMessage)
			}
		}
	}
	return nil
}

// handleInstall handles the install command.
func (r *REPL) handleInstall(args []string) error {
	if len(args) < 1 {
		msg := "Usage: install <image_ref>"
		if r.jsonOutput {
			r.outputJSON("install", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		} else {
			fmt.Fprintln(r.output, msg)
		}
		return fmt.Errorf("missing image_ref argument")
	}

	if err := r.ensureUpdateService(); err != nil {
		if r.jsonOutput {
			r.outputJSON("install", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	resp, err := r.clients.UpdateService.InstallAdapter(ctx, args[0])
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("install", nil, &JSONError{Code: "INSTALL_FAILED", Message: err.Error()})
		}
		return err
	}

	result := map[string]interface{}{
		"adapter_id": resp.AdapterId,
		"state":      resp.State.String(),
	}
	if r.jsonOutput {
		r.outputJSON("install", result, nil)
	} else {
		fmt.Fprintln(r.output, "Adapter installation initiated")
		fmt.Fprintf(r.output, "  Adapter ID: %s\n", resp.AdapterId)
		fmt.Fprintf(r.output, "  State: %s\n", resp.State.String())
	}
	return nil
}

// handleUninstall handles the uninstall command.
func (r *REPL) handleUninstall(args []string) error {
	if len(args) < 1 {
		msg := "Usage: uninstall <adapter_id>"
		if r.jsonOutput {
			r.outputJSON("uninstall", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		} else {
			fmt.Fprintln(r.output, msg)
		}
		return fmt.Errorf("missing adapter_id argument")
	}

	if err := r.ensureUpdateService(); err != nil {
		if r.jsonOutput {
			r.outputJSON("uninstall", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	if err := r.clients.UpdateService.UninstallAdapter(ctx, args[0]); err != nil {
		if r.jsonOutput {
			r.outputJSON("uninstall", nil, &JSONError{Code: "UNINSTALL_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("uninstall", map[string]string{"adapter_id": args[0], "status": "removed"}, nil)
	} else {
		fmt.Fprintf(r.output, "Adapter %s uninstalled successfully\n", args[0])
	}
	return nil
}

// handleStart handles the start command.
func (r *REPL) handleStart(args []string) error {
	if len(args) < 1 {
		msg := "Usage: start <zone_id>"
		if r.jsonOutput {
			r.outputJSON("start", nil, &JSONError{Code: "INVALID_ARGS", Message: msg})
		} else {
			fmt.Fprintln(r.output, msg)
		}
		return fmt.Errorf("missing zone_id argument")
	}

	if err := r.ensureAdaptor(); err != nil {
		if r.jsonOutput {
			r.outputJSON("start", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	resp, err := r.clients.Adaptor.StartSession(ctx, args[0])
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("start", nil, &JSONError{Code: "START_FAILED", Message: err.Error()})
		}
		return err
	}

	if !resp.Success {
		errMsg := resp.ErrorMessage
		if errMsg == "" {
			errMsg = "Session start failed"
		}
		if r.jsonOutput {
			r.outputJSON("start", nil, &JSONError{Code: "START_FAILED", Message: errMsg})
		}
		return errors.New(errMsg)
	}

	result := map[string]interface{}{
		"session_id": resp.SessionID,
		"state":      resp.State,
		"success":    resp.Success,
	}
	if r.jsonOutput {
		r.outputJSON("start", result, nil)
	} else {
		fmt.Fprintln(r.output, "Parking session started")
		fmt.Fprintf(r.output, "  Session ID: %s\n", resp.SessionID)
		fmt.Fprintf(r.output, "  State: %s\n", resp.State)
	}
	return nil
}

// handleStop handles the stop command.
func (r *REPL) handleStop(args []string) error {
	if err := r.ensureAdaptor(); err != nil {
		if r.jsonOutput {
			r.outputJSON("stop", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	resp, err := r.clients.Adaptor.StopSession(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("stop", nil, &JSONError{Code: "STOP_FAILED", Message: err.Error()})
		}
		return err
	}

	if !resp.Success {
		errMsg := resp.ErrorMessage
		if errMsg == "" {
			errMsg = "Session stop failed"
		}
		if r.jsonOutput {
			r.outputJSON("stop", nil, &JSONError{Code: "STOP_FAILED", Message: errMsg})
		}
		return errors.New(errMsg)
	}

	result := map[string]interface{}{
		"session_id":       resp.SessionID,
		"state":            resp.State,
		"final_cost":       resp.FinalCost,
		"duration_seconds": resp.DurationSeconds,
	}
	if r.jsonOutput {
		r.outputJSON("stop", result, nil)
	} else {
		fmt.Fprintln(r.output, "Parking session stopped")
		fmt.Fprintf(r.output, "  Duration: %s\n", formatDuration(resp.DurationSeconds))
		fmt.Fprintf(r.output, "  Final Cost: %.2f\n", resp.FinalCost)
	}
	return nil
}

// handleSession handles the session command.
func (r *REPL) handleSession(args []string) error {
	if err := r.ensureAdaptor(); err != nil {
		if r.jsonOutput {
			r.outputJSON("session", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	session, err := r.clients.Adaptor.GetSessionStatus(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("session", nil, &JSONError{Code: "STATUS_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("session", session, nil)
	} else {
		if !session.HasActiveSession {
			fmt.Fprintln(r.output, "No active parking session")
			return nil
		}
		fmt.Fprintln(r.output, "Session Status:")
		fmt.Fprintf(r.output, "  Session ID: %s\n", session.SessionID)
		fmt.Fprintf(r.output, "  State: %s\n", session.State)
		if session.StartTimeUnix > 0 {
			fmt.Fprintf(r.output, "  Start Time: %d\n", session.StartTimeUnix)
		}
		fmt.Fprintf(r.output, "  Current Cost: %.2f\n", session.CurrentCost)
		if session.ZoneID != "" {
			fmt.Fprintf(r.output, "  Zone ID: %s\n", session.ZoneID)
		}
	}
	return nil
}

// handleLocks handles the locks command.
func (r *REPL) handleLocks(args []string) error {
	if err := r.ensureLocking(); err != nil {
		if r.jsonOutput {
			r.outputJSON("locks", nil, &JSONError{Code: "CONNECTION_ERROR", Message: err.Error()})
		}
		return err
	}

	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	states, err := r.clients.Locking.GetAllLockStates(ctx)
	if err != nil {
		if r.jsonOutput {
			r.outputJSON("locks", nil, &JSONError{Code: "LOCK_FAILED", Message: err.Error()})
		}
		return err
	}

	if r.jsonOutput {
		r.outputJSON("locks", map[string]interface{}{"doors": states}, nil)
	} else {
		fmt.Fprintln(r.output, "Door Lock States:")
		for _, s := range states {
			lockStatus := "UNLOCKED"
			if s.IsLocked {
				lockStatus = "LOCKED"
			}
			openStatus := "Closed"
			if s.IsOpen {
				openStatus = "Open"
			}
			fmt.Fprintf(r.output, "  %s: %s (%s)\n", s.Door, lockStatus, openStatus)
		}
	}
	return nil
}

// handlePing handles the ping command.
func (r *REPL) handlePing(args []string) error {
	ctx, cancel := context.WithTimeout(context.Background(), r.cfg.Timeout)
	defer cancel()

	results := make([]PingResult, 0, 5)

	// Ping DATA_BROKER
	dbResult := PingResult{Service: "DATA_BROKER", Address: r.cfg.DataBrokerAddr}
	if err := r.ensureDataBroker(); err == nil {
		if err := r.clients.DataBroker.Ping(ctx); err == nil {
			dbResult.Connected = true
		} else {
			dbResult.Error = err.Error()
		}
	} else {
		dbResult.Error = err.Error()
	}
	results = append(results, dbResult)

	// Ping PARKING_FEE_SERVICE
	pfResult := PingResult{Service: "PARKING_FEE_SERVICE", Address: r.cfg.ParkingFeeServiceURL}
	if err := r.clients.ParkingFee.Ping(ctx); err == nil {
		pfResult.Connected = true
	} else {
		pfResult.Error = err.Error()
	}
	results = append(results, pfResult)

	// Ping UPDATE_SERVICE
	usResult := PingResult{Service: "UPDATE_SERVICE", Address: r.cfg.UpdateServiceAddr}
	if err := r.ensureUpdateService(); err == nil {
		if err := r.clients.UpdateService.Ping(ctx); err == nil {
			usResult.Connected = true
		} else {
			usResult.Error = err.Error()
		}
	} else {
		usResult.Error = err.Error()
	}
	results = append(results, usResult)

	// Ping PARKING_ADAPTOR
	paResult := PingResult{Service: "PARKING_ADAPTOR", Address: r.cfg.ParkingAdaptorAddr}
	if err := r.ensureAdaptor(); err == nil {
		if err := r.clients.Adaptor.Ping(ctx); err == nil {
			paResult.Connected = true
		} else {
			paResult.Error = err.Error()
		}
	} else {
		paResult.Error = err.Error()
	}
	results = append(results, paResult)

	// Ping LOCKING_SERVICE
	lsResult := PingResult{Service: "LOCKING_SERVICE", Address: r.cfg.LockingServiceAddr}
	if err := r.ensureLocking(); err == nil {
		if err := r.clients.Locking.Ping(ctx); err == nil {
			lsResult.Connected = true
		} else {
			lsResult.Error = err.Error()
		}
	} else {
		lsResult.Error = err.Error()
	}
	results = append(results, lsResult)

	if r.jsonOutput {
		r.outputJSON("ping", map[string]interface{}{"services": results}, nil)
	} else {
		fmt.Fprintln(r.output, "Service Connectivity:")
		for _, res := range results {
			status := "OK"
			if !res.Connected {
				status = "OFFLINE"
			}
			fmt.Fprintf(r.output, "  %-20s %s (%s)\n", res.Service+":", status, res.Address)
			if res.Error != "" && !res.Connected {
				fmt.Fprintf(r.output, "    Error: %s\n", res.Error)
			}
		}
	}
	return nil
}

// handleHelp handles the help command.
func (r *REPL) handleHelp(args []string) error {
	if r.jsonOutput {
		commands := make([]map[string]string, 0, len(r.commands))
		for _, cmd := range r.commands {
			if cmd.Name == "exit" {
				continue
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
		fmt.Fprintln(r.output, "  Location and Zone:")
		fmt.Fprintln(r.output, "    location [<lat> <lng>]  Get or set vehicle location")
		fmt.Fprintln(r.output, "    zone                    Look up parking zone at current location")
		fmt.Fprintln(r.output, "")
		fmt.Fprintln(r.output, "  Adapter Management:")
		fmt.Fprintln(r.output, "    adapters                List installed adapters")
		fmt.Fprintln(r.output, "    install <image_ref>     Install an adapter from registry")
		fmt.Fprintln(r.output, "    uninstall <adapter_id>  Uninstall an adapter")
		fmt.Fprintln(r.output, "")
		fmt.Fprintln(r.output, "  Session Management:")
		fmt.Fprintln(r.output, "    start <zone_id>         Start a parking session")
		fmt.Fprintln(r.output, "    stop                    Stop the current parking session")
		fmt.Fprintln(r.output, "    session                 Get current session status")
		fmt.Fprintln(r.output, "")
		fmt.Fprintln(r.output, "  Vehicle Status:")
		fmt.Fprintln(r.output, "    locks                   Display door lock states")
		fmt.Fprintln(r.output, "")
		fmt.Fprintln(r.output, "  Utilities:")
		fmt.Fprintln(r.output, "    ping                    Test connectivity to all services")
		fmt.Fprintln(r.output, "    help                    Display this help message")
		fmt.Fprintln(r.output, "    quit                    Exit the CLI")
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

// formatDuration formats seconds into a human-readable duration.
func formatDuration(seconds int64) string {
	d := time.Duration(seconds) * time.Second
	h := d / time.Hour
	d -= h * time.Hour
	m := d / time.Minute
	d -= m * time.Minute
	s := d / time.Second
	if h > 0 {
		return fmt.Sprintf("%dh %dm %ds", h, m, s)
	}
	if m > 0 {
		return fmt.Sprintf("%dm %ds", m, s)
	}
	return fmt.Sprintf("%ds", s)
}

// GetCommands returns all registered commands.
func (r *REPL) GetCommands() map[string]Command {
	return r.commands
}
