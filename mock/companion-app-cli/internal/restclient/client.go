// Package restclient provides an HTTP client wrapper with bearer token support.
package restclient

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client wraps an HTTP client with a base URL, bearer token, and timeout.
type Client struct {
	BaseURL     string
	BearerToken string
	HTTPClient  *http.Client
}

// New creates a new REST client with the given base URL, bearer token, and a 10-second timeout.
func New(baseURL, bearerToken string) *Client {
	return &Client{
		BaseURL:     baseURL,
		BearerToken: bearerToken,
		HTTPClient: &http.Client{
			Timeout: 10 * time.Second,
		},
	}
}

// setAuth adds the Authorization header if a bearer token is configured.
func (c *Client) setAuth(req *http.Request) {
	if c.BearerToken != "" {
		req.Header.Set("Authorization", "Bearer "+c.BearerToken)
	}
}

// Get performs an HTTP GET request to the given path and returns the response body.
func (c *Client) Get(path string) ([]byte, error) {
	url := c.BaseURL + path
	req, err := http.NewRequest(http.MethodGet, url, nil)
	if err != nil {
		return nil, fmt.Errorf("GET %s: %w", url, err)
	}
	c.setAuth(req)

	resp, err := c.HTTPClient.Do(req)
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

// Post performs an HTTP POST request with a JSON body and returns the response body.
func (c *Client) Post(path string, jsonBody []byte) ([]byte, error) {
	url := c.BaseURL + path
	req, err := http.NewRequest(http.MethodPost, url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, fmt.Errorf("POST %s: %w", url, err)
	}
	req.Header.Set("Content-Type", "application/json")
	c.setAuth(req)

	resp, err := c.HTTPClient.Do(req)
	if err != nil {
		return nil, fmt.Errorf("POST %s: %w", url, err)
	}
	defer resp.Body.Close()

	body, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, fmt.Errorf("reading response from %s: %w", url, err)
	}

	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, fmt.Errorf("POST %s: HTTP %d: %s", url, resp.StatusCode, string(body))
	}

	return body, nil
}
