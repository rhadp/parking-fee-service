package cmd

import (
	"fmt"

	"github.com/parking-fee-service/mock/companion-app-cli/internal/output"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/restclient"
)

// RunStatus executes the status subcommand.
// Queries command status from CLOUD_GATEWAY for the specified VIN and command ID.
func RunStatus(args []string, gatewayURL string, bearerToken string) error {
	vin, err := requireFlag(args, "vin")
	if err != nil {
		return err
	}
	commandID, err := requireFlag(args, "command-id")
	if err != nil {
		return err
	}

	client := restclient.New(gatewayURL, bearerToken)
	path := fmt.Sprintf("/vehicles/%s/commands/%s", vin, commandID)
	respBody, err := client.Get(path)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(respBody)
}
