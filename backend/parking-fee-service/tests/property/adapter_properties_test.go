// Package property contains property-based tests for the parking-fee-service.
package property

import (
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"regexp"
	"sort"
	"testing"
	"time"

	"github.com/gorilla/mux"
	"github.com/leanovate/gopter"
	"github.com/leanovate/gopter/gen"
	"github.com/leanovate/gopter/prop"

	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/handler"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/model"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/service"
	"github.com/sdv-parking-demo/backend/parking-fee-service/internal/store"
)

// testDemoAdapter is the demo adapter used for testing.
var testDemoAdapter = model.Adapter{
	AdapterID:    "demo-operator",
	OperatorName: "Demo Parking Operator",
	Version:      "1.0.0",
	ImageRef:     "us-docker.pkg.dev/sdv-demo/adapters/demo-operator:v1.0.0",
	Checksum:     "sha256:0000000000000000000000000000000000000000000000000000000000000000",
	CreatedAt:    time.Now(),
}

// testMultipleAdapters is a set of adapters for testing sorting.
var testMultipleAdapters = []model.Adapter{
	{
		AdapterID:    "zebra-parking",
		OperatorName: "Zebra Parking Co",
		Version:      "1.0.0",
		ImageRef:     "registry.io/zebra:v1",
		Checksum:     "sha256:1111111111111111111111111111111111111111111111111111111111111111",
		CreatedAt:    time.Now(),
	},
	{
		AdapterID:    "alpha-parking",
		OperatorName: "Alpha Parking Inc",
		Version:      "2.0.0",
		ImageRef:     "registry.io/alpha:v2",
		Checksum:     "sha256:2222222222222222222222222222222222222222222222222222222222222222",
		CreatedAt:    time.Now(),
	},
	{
		AdapterID:    "beta-parking",
		OperatorName: "Beta Parking LLC",
		Version:      "1.5.0",
		ImageRef:     "registry.io/beta:v1.5",
		Checksum:     "sha256:3333333333333333333333333333333333333333333333333333333333333333",
		CreatedAt:    time.Now(),
	},
}

// TestProperty4_AdapterListCompletenessAndSorting verifies that adapter list
// has all required fields and is sorted by operator_name.
// Feature: parking-fee-service, Property 4: Adapter List Completeness and Sorting
func TestProperty4_AdapterListCompletenessAndSorting(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	adapterStore := store.NewAdapterStore(testMultipleAdapters)
	adapterService := service.NewAdapterService(adapterStore)
	adapterHandler := handler.NewAdapterHandler(adapterService, nil)

	properties.Property("adapter list is sorted by operator_name", prop.ForAll(
		func(_ int) bool {
			req := httptest.NewRequest("GET", "/api/v1/adapters", nil)
			rec := httptest.NewRecorder()
			adapterHandler.HandleListAdapters(rec, req)

			if rec.Code != http.StatusOK {
				return false
			}

			var response model.AdapterListResponse
			if err := json.Unmarshal(rec.Body.Bytes(), &response); err != nil {
				return false
			}

			// Check all fields are present
			for _, adapter := range response.Adapters {
				if adapter.AdapterID == "" || adapter.OperatorName == "" ||
					adapter.Version == "" || adapter.ImageRef == "" {
					return false
				}
			}

			// Check sorting
			isSorted := sort.SliceIsSorted(response.Adapters, func(i, j int) bool {
				return response.Adapters[i].OperatorName < response.Adapters[j].OperatorName
			})

			return isSorted
		},
		gen.IntRange(0, 100),
	))

	properties.TestingRun(t)
}

// TestProperty5_AdapterDetailsRetrieval verifies that valid adapter_id returns complete details.
// Feature: parking-fee-service, Property 5: Adapter Details Retrieval
func TestProperty5_AdapterDetailsRetrieval(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	adapterStore := store.NewAdapterStore([]model.Adapter{testDemoAdapter})
	adapterService := service.NewAdapterService(adapterStore)
	adapterHandler := handler.NewAdapterHandler(adapterService, nil)

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/adapters/{adapter_id}", adapterHandler.HandleGetAdapter).Methods("GET")

	properties.Property("valid adapter_id returns complete details", prop.ForAll(
		func(_ int) bool {
			req := httptest.NewRequest("GET", "/api/v1/adapters/demo-operator", nil)
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			if rec.Code != http.StatusOK {
				return false
			}

			var adapter model.Adapter
			if err := json.Unmarshal(rec.Body.Bytes(), &adapter); err != nil {
				return false
			}

			return adapter.AdapterID != "" &&
				adapter.OperatorName != "" &&
				adapter.Version != "" &&
				adapter.ImageRef != "" &&
				adapter.Checksum != "" &&
				!adapter.CreatedAt.IsZero()
		},
		gen.IntRange(0, 100),
	))

	properties.TestingRun(t)
}

// TestProperty6_AdapterNotFound verifies that invalid adapter_id returns 404.
// Feature: parking-fee-service, Property 6: Adapter Not Found
func TestProperty6_AdapterNotFound(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	adapterStore := store.NewAdapterStore([]model.Adapter{testDemoAdapter})
	adapterService := service.NewAdapterService(adapterStore)
	adapterHandler := handler.NewAdapterHandler(adapterService, nil)

	router := mux.NewRouter()
	router.HandleFunc("/api/v1/adapters/{adapter_id}", adapterHandler.HandleGetAdapter).Methods("GET")

	properties.Property("invalid adapter_id returns 404", prop.ForAll(
		func(adapterID string) bool {
			if adapterID == "demo-operator" {
				return true // Skip valid ID
			}

			req := httptest.NewRequest("GET", "/api/v1/adapters/"+adapterID, nil)
			rec := httptest.NewRecorder()
			router.ServeHTTP(rec, req)

			return rec.Code == http.StatusNotFound
		},
		gen.AlphaString().SuchThat(func(s string) bool { return len(s) > 0 && s != "demo-operator" }),
	))

	properties.TestingRun(t)
}

// TestProperty7_ChecksumFormatValidation verifies that checksum is sha256: followed by 64 hex chars.
// Feature: parking-fee-service, Property 7: Checksum Format Validation
func TestProperty7_ChecksumFormatValidation(t *testing.T) {
	parameters := gopter.DefaultTestParameters()
	parameters.MinSuccessfulTests = 100

	properties := gopter.NewProperties(parameters)

	// Regex for valid SHA256 checksum format
	checksumRegex := regexp.MustCompile(`^sha256:[0-9a-f]{64}$`)

	adapterStore := store.NewAdapterStore(testMultipleAdapters)
	adapterService := service.NewAdapterService(adapterStore)

	properties.Property("all adapter checksums have valid format", prop.ForAll(
		func(_ int) bool {
			adapters := adapterService.ListAdapters()
			for _, summary := range adapters {
				adapter := adapterService.GetAdapter(summary.AdapterID)
				if adapter == nil {
					return false
				}
				if !checksumRegex.MatchString(adapter.Checksum) {
					return false
				}
			}
			return true
		},
		gen.IntRange(0, 100),
	))

	properties.TestingRun(t)
}
