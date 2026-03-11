package cmd

import (
	"context"
	"fmt"
	"io"
	"os"
	"os/signal"
	"syscall"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/grpcclient"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	updatepb "github.com/parking-fee-service/proto/update_service/v1"
)

// RunWatch executes the watch subcommand.
// It streams adapter state changes from UPDATE_SERVICE until interrupted.
func RunWatch(args []string, serviceAddr string) error {
	conn, err := grpcclient.Dial(serviceAddr)
	if err != nil {
		return err
	}
	defer conn.Close()

	client := updatepb.NewUpdateServiceClient(conn)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle Ctrl+C
	sigCh := make(chan os.Signal, 1)
	signal.Notify(sigCh, syscall.SIGINT, syscall.SIGTERM)
	go func() {
		<-sigCh
		cancel()
	}()

	stream, err := client.WatchAdapterStates(ctx, &updatepb.WatchAdapterStatesRequest{})
	if err != nil {
		return err
	}

	fmt.Fprintln(os.Stderr, "Watching adapter state changes (Ctrl+C to stop)...")
	for {
		event, err := stream.Recv()
		if err == io.EOF {
			return nil
		}
		if err != nil {
			if ctx.Err() != nil {
				// Cancelled by signal
				return nil
			}
			return err
		}

		if printErr := output.PrintJSON(map[string]any{
			"adapter_id": event.GetAdapterId(),
			"old_state":  event.GetOldState().String(),
			"new_state":  event.GetNewState().String(),
			"timestamp":  event.GetTimestamp(),
		}); printErr != nil {
			return printErr
		}
	}
}
