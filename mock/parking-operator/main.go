package main

import (
	"context"
	"flag"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"
)

const defaultPort = "8080"

func main() {
	if len(os.Args) < 2 || os.Args[1] == "--help" || os.Args[1] == "-h" {
		printUsage()
		if len(os.Args) >= 2 && (os.Args[1] == "--help" || os.Args[1] == "-h") {
			os.Exit(0)
		}
		os.Exit(1)
	}

	subcmd := os.Args[1]
	switch subcmd {
	case "serve":
		runServe(os.Args[2:])
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %s\n", subcmd)
		printUsage()
		os.Exit(1)
	}
}

func printUsage() {
	fmt.Fprintln(os.Stderr, "Usage: parking-operator <subcommand> [flags]")
	fmt.Fprintln(os.Stderr, "")
	fmt.Fprintln(os.Stderr, "Subcommands:")
	fmt.Fprintln(os.Stderr, "  serve    Start the parking operator HTTP server")
	fmt.Fprintln(os.Stderr, "")
	fmt.Fprintln(os.Stderr, "Flags:")
	fmt.Fprintln(os.Stderr, "  --help   Show this help message")
}

func runServe(args []string) {
	fs := flag.NewFlagSet("serve", flag.ExitOnError)
	portEnv := os.Getenv("PORT")
	if portEnv == "" {
		portEnv = defaultPort
	}
	port := fs.String("port", portEnv, "HTTP server port")
	fs.Parse(args)

	store := NewSessionStore()

	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", HandleStartParking(store))
	mux.HandleFunc("POST /parking/stop", HandleStopParking(store))
	mux.HandleFunc("GET /parking/status/{session_id}", HandleParkingStatus(store))

	addr := fmt.Sprintf(":%s", *port)
	server := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// Graceful shutdown on SIGTERM/SIGINT
	done := make(chan os.Signal, 1)
	signal.Notify(done, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		slog.Info("parking-operator ready", "port", *port)
		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "error", err)
			os.Exit(1)
		}
	}()

	<-done
	slog.Info("shutting down parking-operator")

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := server.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "error", err)
		os.Exit(1)
	}

	slog.Info("parking-operator stopped")
}

// GetPort returns the configured port (for testing).
func GetPort() string {
	if p := os.Getenv("PORT"); p != "" {
		return p
	}
	return defaultPort
}
