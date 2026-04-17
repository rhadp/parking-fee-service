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
)

func main() {
	args := os.Args[1:]
	if len(args) == 0 || args[0] != "serve" {
		fmt.Fprintf(os.Stderr, "usage: parking-operator serve [--port=<port>]\n")
		os.Exit(1)
	}

	// Resolve port: flag > env > default.
	port := "8080"
	if envPort := os.Getenv("PORT"); envPort != "" {
		port = envPort
	}
	for _, arg := range args[1:] {
		if v, ok := strings.CutPrefix(arg, "--port="); ok {
			port = v
		}
	}

	if _, err := strconv.Atoi(port); err != nil {
		fmt.Fprintf(os.Stderr, "invalid port %q: %v\n", port, err)
		os.Exit(1)
	}

	srv := NewServer()
	httpSrv := &http.Server{
		Addr:    ":" + port,
		Handler: srv.Handler(),
	}

	// Listen for OS termination signals.
	stop := make(chan os.Signal, 1)
	signal.Notify(stop, syscall.SIGTERM, syscall.SIGINT)

	go func() {
		fmt.Fprintf(os.Stderr, "parking-operator listening on :%s\n", port)
		if err := httpSrv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			fmt.Fprintf(os.Stderr, "server error: %v\n", err)
			os.Exit(1)
		}
	}()

	<-stop

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	if err := httpSrv.Shutdown(ctx); err != nil {
		fmt.Fprintf(os.Stderr, "shutdown error: %v\n", err)
		os.Exit(1)
	}
}
