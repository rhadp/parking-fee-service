package handler

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

// adapterHandler holds the dependencies for the GET /operators/{id}/adapter
// endpoint.
type adapterHandler struct {
	store *store.Store
}

// adapterMetadataResponse is the JSON structure for adapter metadata.
type adapterMetadataResponse struct {
	OperatorID     string `json:"operator_id"`
	ImageRef       string `json:"image_ref"`
	ChecksumSHA256 string `json:"checksum_sha256"`
	Version        string `json:"version"`
}

// ServeHTTP handles GET /operators/{id}/adapter.
func (h *adapterHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	// Extract operator ID from URL path.
	// Expected path: /operators/{id}/adapter
	id := extractOperatorID(r.URL.Path)
	if id == "" {
		writeJSONError(w, http.StatusBadRequest, "missing operator ID in path")
		return
	}

	op, ok := h.store.GetOperator(id)
	if !ok {
		writeJSONError(w, http.StatusNotFound,
			fmt.Sprintf("operator not found: %s", id))
		return
	}

	resp := adapterMetadataResponse{
		OperatorID:     op.ID,
		ImageRef:       op.Adapter.ImageRef,
		ChecksumSHA256: op.Adapter.ChecksumSHA256,
		Version:        op.Adapter.Version,
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(resp)
}

// extractOperatorID extracts the operator ID from a URL path like
// /operators/{id}/adapter. Returns empty string if the path doesn't match.
func extractOperatorID(path string) string {
	// Trim leading slash and split: ["operators", "{id}", "adapter"]
	parts := strings.Split(strings.TrimPrefix(path, "/"), "/")
	if len(parts) >= 3 && parts[0] == "operators" && parts[2] == "adapter" {
		return parts[1]
	}
	return ""
}
