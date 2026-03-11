package cmd

import (
	"encoding/json"
	"fmt"

	"github.com/google/uuid"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/output"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/restclient"
)

// RunUnlock executes the unlock subcommand.
// Sends an unlock command to CLOUD_GATEWAY for the specified VIN.
func RunUnlock(args []string, gatewayURL string, bearerToken string) error {
	vin, err := requireFlag(args, "vin")
	if err != nil {
		return err
	}

	payload := map[string]interface{}{
		"command_id": uuid.New().String(),
		"type":       "unlock",
		"doors":      []string{"driver"},
	}

	body, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("marshalling unlock command: %w", err)
	}

	client := restclient.New(gatewayURL, bearerToken)
	path := fmt.Sprintf("/vehicles/%s/commands", vin)
	respBody, err := client.Post(path, body)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(respBody)
}
