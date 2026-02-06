package service

import (
	"context"
	"encoding/json"
	"fmt"
	"io"
	"log/slog"
	"net/http"
	"sync"
	"time"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/model"
)

// ParkingSessionService handles parking session queries by proxying to PARKING_FEE_SERVICE.
type ParkingSessionService struct {
	parkingFeeServiceURL string
	httpClient           *http.Client
	logger               *slog.Logger
	configuredVIN        string

	// Cache with 5-second TTL
	cache     *model.ParkingSession
	cacheTime time.Time
	cacheTTL  time.Duration
	cacheMu   sync.RWMutex
}

// NewParkingSessionService creates a new ParkingSessionService.
func NewParkingSessionService(
	parkingFeeServiceURL string,
	logger *slog.Logger,
	configuredVIN string,
) *ParkingSessionService {
	return &ParkingSessionService{
		parkingFeeServiceURL: parkingFeeServiceURL,
		httpClient: &http.Client{
			Timeout: 10 * time.Second,
		},
		logger:        logger,
		configuredVIN: configuredVIN,
		cacheTTL:      5 * time.Second,
	}
}

// ParkingSessionResult represents the result of a parking session query.
type ParkingSessionResult struct {
	Session *model.ParkingSession
	Error   error
	Code    string // Error code if applicable
}

// GetParkingSession retrieves the current parking session for the configured VIN.
// It proxies the request to PARKING_FEE_SERVICE and caches the result for 5 seconds.
func (s *ParkingSessionService) GetParkingSession(ctx context.Context, vin string) *ParkingSessionResult {
	// Validate VIN
	if vin != s.configuredVIN {
		return &ParkingSessionResult{
			Error: fmt.Errorf("vehicle not found: %s", vin),
			Code:  model.ErrVehicleNotFound,
		}
	}

	// Check cache
	s.cacheMu.RLock()
	if s.cache != nil && time.Since(s.cacheTime) < s.cacheTTL {
		cached := *s.cache
		s.cacheMu.RUnlock()
		return &ParkingSessionResult{Session: &cached}
	}
	s.cacheMu.RUnlock()

	// Query PARKING_FEE_SERVICE
	url := fmt.Sprintf("%s/api/v1/parking/status/active?vehicle_id=%s", s.parkingFeeServiceURL, vin)

	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		s.logger.Error("failed to create request to parking fee service",
			slog.String("error", err.Error()),
		)
		return &ParkingSessionResult{
			Error: fmt.Errorf("internal error"),
			Code:  model.ErrInternalError,
		}
	}

	resp, err := s.httpClient.Do(req)
	if err != nil {
		s.logger.Warn("failed to query parking fee service",
			slog.String("error", err.Error()),
		)
		// Return cached value if available, even if expired
		s.cacheMu.RLock()
		if s.cache != nil {
			cached := *s.cache
			s.cacheMu.RUnlock()
			return &ParkingSessionResult{Session: &cached}
		}
		s.cacheMu.RUnlock()

		return &ParkingSessionResult{
			Error: fmt.Errorf("parking fee service unavailable"),
			Code:  model.ErrInternalError,
		}
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		s.logger.Error("failed to read parking fee service response",
			slog.String("error", err.Error()),
		)
		return &ParkingSessionResult{
			Error: fmt.Errorf("internal error"),
			Code:  model.ErrInternalError,
		}
	}

	// Handle 404 - no active session
	if resp.StatusCode == http.StatusNotFound {
		return &ParkingSessionResult{
			Error: fmt.Errorf("no active parking session for this vehicle"),
			Code:  model.ErrNoActiveSession,
		}
	}

	// Handle other non-200 responses
	if resp.StatusCode != http.StatusOK {
		s.logger.Warn("parking fee service returned error",
			slog.Int("status", resp.StatusCode),
			slog.String("body", string(body)),
		)
		return &ParkingSessionResult{
			Error: fmt.Errorf("parking fee service error"),
			Code:  model.ErrInternalError,
		}
	}

	// Parse response
	var session model.ParkingSession
	if err := json.Unmarshal(body, &session); err != nil {
		s.logger.Error("failed to parse parking fee service response",
			slog.String("error", err.Error()),
			slog.String("body", string(body)),
		)
		return &ParkingSessionResult{
			Error: fmt.Errorf("internal error"),
			Code:  model.ErrInternalError,
		}
	}

	// Update cache
	s.cacheMu.Lock()
	s.cache = &session
	s.cacheTime = time.Now()
	s.cacheMu.Unlock()

	s.logger.Debug("parking session retrieved",
		slog.String("session_id", session.SessionID),
		slog.String("zone_name", session.ZoneName),
	)

	return &ParkingSessionResult{Session: &session}
}

// ClearCache clears the cached parking session.
func (s *ParkingSessionService) ClearCache() {
	s.cacheMu.Lock()
	s.cache = nil
	s.cacheTime = time.Time{}
	s.cacheMu.Unlock()
}
