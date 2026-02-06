package client

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"time"
)

// ZoneInfo represents parking zone information from PARKING_FEE_SERVICE.
type ZoneInfo struct {
	ZoneID          string  `json:"zone_id"`
	OperatorName    string  `json:"operator_name"`
	HourlyRate      float64 `json:"hourly_rate"`
	Currency        string  `json:"currency"`
	AdapterImageRef string  `json:"adapter_image_ref"`
	AdapterChecksum string  `json:"adapter_checksum"`
}

// ZoneLookupResponse represents the zone lookup API response.
type ZoneLookupResponse struct {
	Found bool      `json:"found"`
	Zone  *ZoneInfo `json:"zone,omitempty"`
}

// ParkingFeeClient handles communication with PARKING_FEE_SERVICE REST API.
type ParkingFeeClient struct {
	baseURL    string
	httpClient *http.Client
}

// NewParkingFeeClient creates a new ParkingFeeClient.
func NewParkingFeeClient(baseURL string, timeout time.Duration) *ParkingFeeClient {
	return &ParkingFeeClient{
		baseURL: baseURL,
		httpClient: &http.Client{
			Timeout: timeout,
		},
	}
}

// GetZone looks up the parking zone for the given coordinates.
func (c *ParkingFeeClient) GetZone(ctx context.Context, lat, lng float64) (*ZoneInfo, error) {
	url := fmt.Sprintf("%s/api/v1/zones?lat=%f&lng=%f", c.baseURL, lat, lng)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("cannot connect to %s: %w", c.baseURL, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("failed to read response: %w", err)
	}

	if resp.StatusCode == http.StatusNotFound {
		return nil, nil // No zone found at this location
	}

	if resp.StatusCode >= 400 {
		return nil, fmt.Errorf("HTTP %d: %s", resp.StatusCode, string(body))
	}

	var zoneResp ZoneLookupResponse
	if err := json.Unmarshal(body, &zoneResp); err != nil {
		// Try parsing as direct ZoneInfo
		var zone ZoneInfo
		if err := json.Unmarshal(body, &zone); err != nil {
			return nil, fmt.Errorf("failed to parse response: %w", err)
		}
		return &zone, nil
	}

	if !zoneResp.Found || zoneResp.Zone == nil {
		return nil, nil
	}

	return zoneResp.Zone, nil
}

// Ping tests connectivity to the PARKING_FEE_SERVICE.
func (c *ParkingFeeClient) Ping(ctx context.Context) error {
	url := fmt.Sprintf("%s/health", c.baseURL)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}

	resp, err := c.httpClient.Do(req)
	if err != nil {
		return fmt.Errorf("cannot connect to %s: %w", c.baseURL, err)
	}
	defer resp.Body.Close()

	if resp.StatusCode >= 400 {
		return fmt.Errorf("HTTP %d from %s", resp.StatusCode, url)
	}

	return nil
}

// GetBaseURL returns the base URL.
func (c *ParkingFeeClient) GetBaseURL() string {
	return c.baseURL
}
