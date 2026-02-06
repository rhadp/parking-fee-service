// Package client provides gRPC clients for PARKING_CLI.
package client

import (
	"context"
	"fmt"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
	"google.golang.org/protobuf/types/known/timestamppb"

	"github.com/sdv-parking-demo/backend/gen/services/databroker"
	"github.com/sdv-parking-demo/backend/gen/vss"
)

// VSS signal path for location.
const (
	SignalLocation = "Vehicle.CurrentLocation"
)

// DataBrokerClient handles communication with DATA_BROKER gRPC service.
type DataBrokerClient struct {
	conn    *grpc.ClientConn
	client  databroker.DataBrokerClient
	address string
}

// NewDataBrokerClient creates a new DataBrokerClient.
func NewDataBrokerClient(address string, timeout time.Duration) (*DataBrokerClient, error) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, address,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to DATA_BROKER at %s: %w", address, err)
	}

	return &DataBrokerClient{
		conn:    conn,
		client:  databroker.NewDataBrokerClient(conn),
		address: address,
	}, nil
}

// SetLocation sets the vehicle location signal.
func (c *DataBrokerClient) SetLocation(ctx context.Context, lat, lng float64) error {
	_, err := c.client.SetSignal(ctx, &databroker.SetSignalRequest{
		SignalPath: SignalLocation,
		Signal: &vss.VehicleSignal{
			Signal: &vss.VehicleSignal_Location{
				Location: &vss.Location{
					Latitude:  lat,
					Longitude: lng,
					Timestamp: timestamppb.Now(),
				},
			},
		},
	})
	if err != nil {
		return fmt.Errorf("failed to set location: %w", err)
	}

	return nil
}

// GetLocation retrieves the current vehicle location.
func (c *DataBrokerClient) GetLocation(ctx context.Context) (lat, lng float64, err error) {
	resp, err := c.client.GetSignal(ctx, &databroker.GetSignalRequest{
		SignalPaths: []string{SignalLocation},
	})
	if err != nil {
		return 0, 0, fmt.Errorf("failed to get location: %w", err)
	}

	for _, signal := range resp.Signals {
		if loc := signal.GetLocation(); loc != nil {
			lat = loc.Latitude
			lng = loc.Longitude
			break
		}
	}

	return lat, lng, nil
}

// Ping tests connectivity to the DATA_BROKER.
func (c *DataBrokerClient) Ping(ctx context.Context) error {
	_, err := c.client.GetSignal(ctx, &databroker.GetSignalRequest{
		SignalPaths: []string{SignalLocation},
	})
	return err
}

// Close closes the gRPC connection.
func (c *DataBrokerClient) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// GetAddress returns the service address.
func (c *DataBrokerClient) GetAddress() string {
	return c.address
}
