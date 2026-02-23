package main

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"

	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/handler"
	"github.com/rhadp/parking-fee-service/backend/parking-fee-service/internal/store"
)

func TestHealthEndpoint(t *testing.T) {
	s := store.NewDefaultStore()
	router := handler.NewRouter(s, []string{"demo-token-1"}, 100)

	req := httptest.NewRequest(http.MethodGet, "/health", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	if w.Code != http.StatusOK {
		t.Errorf("expected status 200, got %d", w.Code)
	}
	body := w.Body.String()
	if body == "" {
		t.Error("expected non-empty response body")
	}
	if !strings.Contains(body, `"status"`) || !strings.Contains(body, `"ok"`) {
		t.Errorf("expected health response with status ok, got %q", body)
	}
}
