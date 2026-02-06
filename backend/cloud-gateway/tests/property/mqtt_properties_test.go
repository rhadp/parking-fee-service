package property

import (
	"testing"
	"time"

	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/cloud-gateway/internal/mqtt"
)

// TestExponentialBackoffCalculation tests Property 13: Exponential Backoff Calculation.
// For any reconnection attempt number N, the backoff delay SHALL be
// min(initial_delay * 2^N, max_delay) where initial_delay is 1 second
// and max_delay is 30 seconds.
func TestExponentialBackoffCalculation(t *testing.T) {
	// Feature: cloud-gateway, Property 13: Exponential Backoff Calculation
	// Validates: Requirements 1.4

	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	initialBackoff := 1 * time.Second
	maxBackoff := 30 * time.Second

	properties.Property("backoff never exceeds max backoff", prop.ForAll(
		func(attempt int) bool {
			backoff := mqtt.CalculateBackoff(attempt, initialBackoff, maxBackoff)
			return backoff <= maxBackoff
		},
		gen.IntRange(0, 100),
	))

	properties.Property("backoff doubles with each attempt until max", prop.ForAll(
		func(attempt int) bool {
			if attempt <= 0 {
				return mqtt.CalculateBackoff(attempt, initialBackoff, maxBackoff) == initialBackoff
			}

			current := mqtt.CalculateBackoff(attempt, initialBackoff, maxBackoff)
			previous := mqtt.CalculateBackoff(attempt-1, initialBackoff, maxBackoff)

			// Either doubled, or both at max
			if previous >= maxBackoff {
				return current == maxBackoff
			}
			expected := previous * 2
			if expected > maxBackoff {
				expected = maxBackoff
			}
			return current == expected
		},
		gen.IntRange(0, 20),
	))

	properties.Property("attempt 0 returns initial backoff", prop.ForAll(
		func(initialMs, maxMs int) bool {
			initial := time.Duration(initialMs) * time.Millisecond
			max := time.Duration(maxMs) * time.Millisecond
			if initial > max {
				initial, max = max, initial
			}
			if initial <= 0 {
				initial = 1 * time.Millisecond
			}
			if max <= 0 {
				max = 1 * time.Millisecond
			}

			backoff := mqtt.CalculateBackoff(0, initial, max)
			return backoff == initial
		},
		gen.IntRange(1, 1000),
		gen.IntRange(1, 10000),
	))

	properties.Property("backoff is monotonically non-decreasing", prop.ForAll(
		func(attempt int) bool {
			if attempt <= 0 {
				return true
			}
			current := mqtt.CalculateBackoff(attempt, initialBackoff, maxBackoff)
			previous := mqtt.CalculateBackoff(attempt-1, initialBackoff, maxBackoff)
			return current >= previous
		},
		gen.IntRange(0, 50),
	))

	properties.Property("specific values match expected exponential series", prop.ForAll(
		func(attempt int) bool {
			backoff := mqtt.CalculateBackoff(attempt, initialBackoff, maxBackoff)

			// Expected: 1s, 2s, 4s, 8s, 16s, 30s (capped), 30s...
			expected := initialBackoff
			for i := 0; i < attempt; i++ {
				expected *= 2
				if expected > maxBackoff {
					expected = maxBackoff
					break
				}
			}

			return backoff == expected
		},
		gen.IntRange(0, 10),
	))

	properties.TestingRun(t)
}
