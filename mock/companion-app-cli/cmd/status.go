package cmd

import (
	"fmt"

	"github.com/parking-fee-service/mock/companion-app-cli/internal/output"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/restclient"
)

// RunStatus executes the status subcommand.
// Queries vehicle status from CLOUD_GATEWAY for the specified VIN.
func RunStatus(args []string, gatewayURL string, bearerToken string) error {
	vin, err := requireFlag(args, "vin")
	if err != nil {
		return err
	}

	client := restclient.New(gatewayURL, bearerToken)
	path := fmt.Sprintf("/vehicles/%s/status", vin)
	respBody, err := client.Get(path)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(respBody)
}
