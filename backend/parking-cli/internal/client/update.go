package client

import (
	"context"
	"fmt"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/sdv-parking-demo/backend/gen/services/update"
)

// AdapterInfo represents adapter information.
type AdapterInfo struct {
	AdapterID    string
	ImageRef     string
	Version      string
	State        string
	ErrorMessage string
}

// UpdateServiceClient handles communication with UPDATE_SERVICE gRPC service.
type UpdateServiceClient struct {
	conn    *grpc.ClientConn
	client  update.UpdateServiceClient
	address string
}

// NewUpdateServiceClient creates a new UpdateServiceClient.
func NewUpdateServiceClient(address string, timeout time.Duration) (*UpdateServiceClient, error) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, address,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to UPDATE_SERVICE at %s: %w", address, err)
	}

	return &UpdateServiceClient{
		conn:    conn,
		client:  update.NewUpdateServiceClient(conn),
		address: address,
	}, nil
}

// ListAdapters lists installed adapters.
func (c *UpdateServiceClient) ListAdapters(ctx context.Context) ([]*AdapterInfo, error) {
	resp, err := c.client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	if err != nil {
		return nil, fmt.Errorf("failed to list adapters: %w", err)
	}

	adapters := make([]*AdapterInfo, 0, len(resp.Adapters))
	for _, a := range resp.Adapters {
		adapters = append(adapters, &AdapterInfo{
			AdapterID:    a.AdapterId,
			ImageRef:     a.ImageRef,
			Version:      a.Version,
			State:        adapterStateToString(a.State),
			ErrorMessage: a.ErrorMessage,
		})
	}

	return adapters, nil
}

// InstallAdapter requests adapter installation.
func (c *UpdateServiceClient) InstallAdapter(ctx context.Context, imageRef string) (*update.InstallAdapterResponse, error) {
	resp, err := c.client.InstallAdapter(ctx, &update.InstallAdapterRequest{
		ImageRef: imageRef,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to install adapter: %w", err)
	}

	return resp, nil
}

// UninstallAdapter requests adapter removal.
func (c *UpdateServiceClient) UninstallAdapter(ctx context.Context, adapterID string) error {
	_, err := c.client.UninstallAdapter(ctx, &update.UninstallAdapterRequest{
		AdapterId: adapterID,
	})
	if err != nil {
		return fmt.Errorf("failed to uninstall adapter: %w", err)
	}

	return nil
}

// Ping tests connectivity to the UPDATE_SERVICE.
func (c *UpdateServiceClient) Ping(ctx context.Context) error {
	_, err := c.client.ListAdapters(ctx, &update.ListAdaptersRequest{})
	return err
}

// Close closes the gRPC connection.
func (c *UpdateServiceClient) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// GetAddress returns the service address.
func (c *UpdateServiceClient) GetAddress() string {
	return c.address
}

// adapterStateToString converts adapter state to string.
func adapterStateToString(state update.AdapterState) string {
	switch state {
	case update.AdapterState_ADAPTER_STATE_DOWNLOADING:
		return "DOWNLOADING"
	case update.AdapterState_ADAPTER_STATE_INSTALLING:
		return "INSTALLING"
	case update.AdapterState_ADAPTER_STATE_RUNNING:
		return "RUNNING"
	case update.AdapterState_ADAPTER_STATE_STOPPED:
		return "STOPPED"
	case update.AdapterState_ADAPTER_STATE_ERROR:
		return "ERROR"
	default:
		return "UNKNOWN"
	}
}
