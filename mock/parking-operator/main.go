package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/rhadp/parking-fee-service/mock/parking-operator/server"
)

func main() {
	if len(os.Args) < 2 || os.Args[1] != "serve" {
		fmt.Fprintf(os.Stderr, "Usage: %s serve [--port=PORT]\n", os.Args[0])
		os.Exit(1)
	}

	port := resolvePort()

	handler := server.New()
	srv := &http.Server{
		Addr:    fmt.Sprintf(":%d", port),
		Handler: handler,
	}

	// Channel to signal shutdown completion.
	done := make(chan struct{})

	// Graceful shutdown on SIGTERM/SIGINT.
	go func() {
		sigCh := make(chan os.Signal, 1)
		signal.Notify(sigCh, syscall.SIGTERM, syscall.SIGINT)
		<-sigCh

		fmt.Fprintf(os.Stderr, "Shutting down...\n")

		ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
		defer cancel()

		if err := srv.Shutdown(ctx); err != nil {
			fmt.Fprintf(os.Stderr, "Error during shutdown: %v\n", err)
		}
		close(done)
	}()

	fmt.Fprintf(os.Stderr, "parking-operator listening on :%d\n", port)
	if err := srv.ListenAndServe(); err != http.ErrServerClosed {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}

	<-done
}

// resolvePort determines the server port from --port flag, PORT env var, or default 9090.
func resolvePort() int {
	// Check CLI args for --port=N
	for _, arg := range os.Args[2:] {
		if strings.HasPrefix(arg, "--port=") {
			val := strings.TrimPrefix(arg, "--port=")
			p, err := strconv.Atoi(val)
			if err != nil {
				fmt.Fprintf(os.Stderr, "Error: invalid port %q: %v\n", val, err)
				os.Exit(1)
			}
			return p
		}
	}

	// Check PORT env var.
	if envPort := os.Getenv("PORT"); envPort != "" {
		p, err := strconv.Atoi(envPort)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error: invalid PORT env var %q: %v\n", envPort, err)
			os.Exit(1)
		}
		return p
	}

	return 9090
}
