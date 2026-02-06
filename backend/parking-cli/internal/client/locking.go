package client

import (
	"context"
	"fmt"
	"time"

	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"

	"github.com/sdv-parking-demo/backend/gen/services/locking"
)

// LockStateInfo represents door lock state.
type LockStateInfo struct {
	Door     string
	IsLocked bool
	IsOpen   bool
}

// LockingServiceClient handles communication with LOCKING_SERVICE gRPC service.
type LockingServiceClient struct {
	conn    *grpc.ClientConn
	client  locking.LockingServiceClient
	address string
}

// NewLockingServiceClient creates a new LockingServiceClient.
func NewLockingServiceClient(address string, timeout time.Duration) (*LockingServiceClient, error) {
	ctx, cancel := context.WithTimeout(context.Background(), timeout)
	defer cancel()

	conn, err := grpc.DialContext(ctx, address,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
		grpc.WithBlock(),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to LOCKING_SERVICE at %s: %w", address, err)
	}

	return &LockingServiceClient{
		conn:    conn,
		client:  locking.NewLockingServiceClient(conn),
		address: address,
	}, nil
}

// GetLockState retrieves the lock state for a door.
func (c *LockingServiceClient) GetLockState(ctx context.Context, door locking.Door) (*LockStateInfo, error) {
	resp, err := c.client.GetLockState(ctx, &locking.GetLockStateRequest{
		Door: door,
	})
	if err != nil {
		return nil, fmt.Errorf("failed to get lock state: %w", err)
	}

	return &LockStateInfo{
		Door:     doorToString(resp.Door),
		IsLocked: resp.IsLocked,
		IsOpen:   resp.IsOpen,
	}, nil
}

// GetAllLockStates retrieves lock states for all doors.
func (c *LockingServiceClient) GetAllLockStates(ctx context.Context) ([]*LockStateInfo, error) {
	doors := []locking.Door{
		locking.Door_DOOR_DRIVER,
		locking.Door_DOOR_PASSENGER,
		locking.Door_DOOR_REAR_LEFT,
		locking.Door_DOOR_REAR_RIGHT,
	}

	states := make([]*LockStateInfo, 0, len(doors))
	for _, door := range doors {
		state, err := c.GetLockState(ctx, door)
		if err != nil {
			return nil, err
		}
		states = append(states, state)
	}

	return states, nil
}

// Ping tests connectivity to the LOCKING_SERVICE.
func (c *LockingServiceClient) Ping(ctx context.Context) error {
	_, err := c.client.GetLockState(ctx, &locking.GetLockStateRequest{
		Door: locking.Door_DOOR_DRIVER,
	})
	return err
}

// Close closes the gRPC connection.
func (c *LockingServiceClient) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// GetAddress returns the service address.
func (c *LockingServiceClient) GetAddress() string {
	return c.address
}

// doorToString converts door enum to string.
func doorToString(door locking.Door) string {
	switch door {
	case locking.Door_DOOR_DRIVER:
		return "Driver"
	case locking.Door_DOOR_PASSENGER:
		return "Passenger"
	case locking.Door_DOOR_REAR_LEFT:
		return "Rear Left"
	case locking.Door_DOOR_REAR_RIGHT:
		return "Rear Right"
	case locking.Door_DOOR_ALL:
		return "All"
	default:
		return "Unknown"
	}
}
