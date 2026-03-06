package restclient

import (
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client is an HTTP client wrapper with a default timeout.
type Client struct {
	http *http.Client
}

// New creates a new REST client with a 10-second timeout.
func New() *Client {
	return &Client{
		http: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

// Get performs an HTTP GET and returns the response body.
func (c *Client) Get(url string) ([]byte, error) {
	resp, err := c.http.Get(url)
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
