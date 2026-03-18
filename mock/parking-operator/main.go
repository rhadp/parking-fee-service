package main

import (
	"context"
	"fmt"
	"log/slog"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/mock/parking-operator/handler"
	"github.com/rhadp/parking-fee-service/mock/parking-operator/store"
)

// getPort returns the port to listen on. It reads from the PORT environment
// variable first, then the --port flag, then falls back to 8080.
// Satisfies: 09-REQ-5.4
func getPort(args []string) int {
	// Check --port=N flag in args.
	for _, arg := range args {
		if len(arg) > 7 && arg[:7] == "--port=" {
			if n, err := strconv.Atoi(arg[7:]); err == nil {
				return n
			}
		}
	}
	// Check PORT environment variable.
	if v := os.Getenv("PORT"); v != "" {
		if n, err := strconv.Atoi(v); err == nil {
			return n
		}
	}
	return 8080
}

func main() {
	args := os.Args[1:]

	if len(args) == 0 || args[0] == "--help" || args[0] == "-h" {
		fmt.Fprintln(os.Stderr, "Usage: parking-operator <subcommand> [options]")
		fmt.Fprintln(os.Stderr, "")
		fmt.Fprintln(os.Stderr, "Subcommands:")
		fmt.Fprintln(os.Stderr, "  serve [--port=N]   Start the HTTP server")
		fmt.Fprintln(os.Stderr, "")
		fmt.Fprintln(os.Stderr, "Environment variables:")
		fmt.Fprintln(os.Stderr, "  PORT   Listen port (default 8080)")
		if len(args) > 0 && (args[0] == "--help" || args[0] == "-h") {
			os.Exit(0)
		}
		os.Exit(1)
	}

	switch args[0] {
	case "serve":
		port := getPort(args[1:])
		runServer(port)
	default:
		fmt.Fprintf(os.Stderr, "unknown subcommand: %q\n", args[0])
		fmt.Fprintln(os.Stderr, "Run 'parking-operator --help' for usage.")
		os.Exit(1)
	}
}

// runServer starts the HTTP server on the given port and blocks until a
// shutdown signal (SIGTERM or SIGINT) is received, then performs a graceful
// shutdown. Satisfies: 09-REQ-2.1, 09-REQ-2.5
func runServer(port int) {
	s := store.NewStore()

	mux := http.NewServeMux()
	mux.HandleFunc("POST /parking/start", handler.StartHandler(s))
	mux.HandleFunc("POST /parking/stop", handler.StopHandler(s))
	mux.HandleFunc("GET /parking/sessions", handler.ListSessionsHandler(s))
	mux.HandleFunc("GET /parking/status/{session_id}", handler.StatusHandler(s))

	addr := fmt.Sprintf(":%d", port)
	srv := &http.Server{
		Addr:    addr,
		Handler: mux,
	}

	// Listen for OS shutdown signals.
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		slog.Info("parking-operator ready", "port", port)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			slog.Error("server error", "err", err)
			os.Exit(1)
		}
	}()

	<-quit
	slog.Info("parking-operator shutting down")

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := srv.Shutdown(ctx); err != nil {
		slog.Error("shutdown error", "err", err)
		os.Exit(1)
	}
	slog.Info("parking-operator stopped")
}
