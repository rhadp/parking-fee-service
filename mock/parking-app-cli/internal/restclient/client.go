// Package restclient provides an HTTP client wrapper with timeout for REST calls.
package restclient

import (
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client wraps an HTTP client with a base URL and timeout.
type Client struct {
	BaseURL    string
	HTTPClient *http.Client
}

// New creates a new REST client with the given base URL and a 10-second timeout.
func New(baseURL string) *Client {
	return &Client{
		BaseURL: baseURL,
		HTTPClient: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

// Get performs an HTTP GET request to the given path and returns the response body.
func (c *Client) Get(path string) ([]byte, error) {
	url := c.BaseURL + path
	resp, err := c.HTTPClient.Get(url)
	if err != nil {
		return nil, fmt.Errorf("GET %s: %w", url, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response from %s: %w", url, err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("GET %s: HTTP %d: %s", url, resp.StatusCode, string(body))
	}

	return body, nil
}
