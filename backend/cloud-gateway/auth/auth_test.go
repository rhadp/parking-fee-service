package auth_test

import (
	"testing"

	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/auth"
	"github.com/rhadp/parking-fee-service/backend/cloud-gateway/model"
)

// TS-06-17: Token-VIN Authorization
// A token is verified against the VIN in the URL path.
func TestTokenVINAuthorization(t *testing.T) {
	a := auth.NewAuthenticator([]model.TokenMapping{
		{Token: "t1", VIN: "V1"},
	})
	if a == nil {
		t.Fatal("NewAuthenticator returned nil")
	}
	if !a.AuthorizeVIN("t1", "V1") {
		t.Error("AuthorizeVIN(t1, V1) = false, want true")
	}
	if a.AuthorizeVIN("t1", "V2") {
		t.Error("AuthorizeVIN(t1, V2) = true, want false")
	}
}

// ValidateToken with valid Bearer header.
func TestValidateToken_Valid(t *testing.T) {
	token, err := auth.ValidateToken("Bearer my-token-123")
	if err != nil {
		t.Fatalf("ValidateToken(valid) returned error: %v", err)
	}
	if token != "my-token-123" {
		t.Errorf("token = %q, want %q", token, "my-token-123")
	}
}

// ValidateToken with empty header returns error.
func TestValidateToken_Empty(t *testing.T) {
	_, err := auth.ValidateToken("")
	if err == nil {
		t.Error("ValidateToken(empty) returned nil error, want error")
	}
}

// ValidateToken with non-Bearer prefix returns error.
func TestValidateToken_NonBearer(t *testing.T) {
	_, err := auth.ValidateToken("Basic abc123")
	if err == nil {
		t.Error("ValidateToken(Basic) returned nil error, want error")
	}
}

// ValidateToken with just "Bearer " (no token) returns error.
func TestValidateToken_BearerOnly(t *testing.T) {
	_, err := auth.ValidateToken("Bearer ")
	if err == nil {
		t.Error("ValidateToken('Bearer ') returned nil error, want error")
	}
}

// TS-06-P2: Authentication Enforcement Property
// For any request, missing/malformed tokens yield error and mismatched VINs yield false.
func TestPropertyAuthEnforcement(t *testing.T) {
	// Malformed headers
	malformedHeaders := []string{
		"",
		"Basic abc",
		"bearer token",
		"Token abc",
		"Bearertoken",
	}
	for _, h := range malformedHeaders {
		_, err := auth.ValidateToken(h)
		if err == nil {
			t.Errorf("ValidateToken(%q) returned nil error, want error", h)
		}
	}

	// Valid headers extract correct token
	validHeaders := []struct {
		header string
		token  string
	}{
		{"Bearer abc", "abc"},
		{"Bearer demo-token-car1", "demo-token-car1"},
		{"Bearer x", "x"},
	}
	for _, tc := range validHeaders {
		token, err := auth.ValidateToken(tc.header)
		if err != nil {
			t.Errorf("ValidateToken(%q) returned error: %v", tc.header, err)
			continue
		}
		if token != tc.token {
			t.Errorf("ValidateToken(%q) = %q, want %q", tc.header, token, tc.token)
		}
	}

	// VIN authorization: only matching pairs return true
	a := auth.NewAuthenticator([]model.TokenMapping{
		{Token: "t1", VIN: "V1"},
		{Token: "t2", VIN: "V2"},
	})
	if a == nil {
		t.Fatal("NewAuthenticator returned nil")
	}

	cases := []struct {
		token string
		vin   string
		want  bool
	}{
		{"t1", "V1", true},
		{"t2", "V2", true},
		{"t1", "V2", false},
		{"t2", "V1", false},
		{"t1", "V3", false},
		{"unknown", "V1", false},
	}
	for _, tc := range cases {
		got := a.AuthorizeVIN(tc.token, tc.vin)
		if got != tc.want {
			t.Errorf("AuthorizeVIN(%q, %q) = %v, want %v", tc.token, tc.vin, got, tc.want)
		}
	}
}
