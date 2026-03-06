package cmd

import (
	"encoding/json"
	"flag"
	"fmt"
	"os"

	"github.com/google/uuid"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/config"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/output"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/restclient"
)

func runUnlock(args []string) error {
	fs := flag.NewFlagSet("unlock", flag.ContinueOnError)
	vin := fs.String("vin", "", "Vehicle Identification Number (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *vin == "" {
		return fmt.Errorf("usage: companion-app-cli unlock --vin=<vin>\n  --vin is required")
	}

	token := config.BearerToken()
	if token == "" {
		fmt.Fprintln(os.Stderr, "Warning: BEARER_TOKEN not set, proceeding without auth header")
	}

	payload := map[string]interface{}{
		"command_id": uuid.New().String(),
		"type":       "unlock",
		"doors":      []string{"driver"},
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("marshalling request body: %w", err)
	}

	baseURL := config.CloudGatewayURL()
	url := fmt.Sprintf("%s/vehicles/%s/commands", baseURL, *vin)

	client := restclient.New(token)
	resp, err := client.Post(url, body)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(resp)
}
