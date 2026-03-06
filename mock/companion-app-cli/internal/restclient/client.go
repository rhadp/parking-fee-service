package restclient

import (
	"bytes"
	"fmt"
	"io"
	"net/http"
	"time"
)

// Client is an HTTP client wrapper with bearer token support.
type Client struct {
	http  *http.Client
	token string
}

// New creates a new REST client with a 10-second timeout and optional bearer token.
func New(bearerToken string) *Client {
	return &Client{
		http: &http.Client{
			Timeout: 10 * time.Second,
		},
		token: bearerToken,
	}
}

// Get performs an HTTP GET and returns the response body.
func (c *Client) Get(url string) ([]byte, error) {
	req, err := http.NewRequest("GET", url, nil)
	if err != nil {
		return nil, fmt.Errorf("GET %s: %w", url, err)
	}
	c.setAuth(req)

	resp, err := c.http.Do(req)
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

// Post performs an HTTP POST with a JSON body and returns the response body.
func (c *Client) Post(url string, jsonBody []byte) ([]byte, error) {
	req, err := http.NewRequest("POST", url, bytes.NewReader(jsonBody))
	if err != nil {
		return nil, fmt.Errorf("POST %s: %w", url, err)
	}
	req.Header.Set("Content-Type", "application/json")
	c.setAuth(req)

	resp, err := c.http.Do(req)
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

// setAuth adds the Authorization header if a bearer token is configured.
func (c *Client) setAuth(req *http.Request) {
	if c.token != "" {
		req.Header.Set("Authorization", "Bearer "+c.token)
	}
}
