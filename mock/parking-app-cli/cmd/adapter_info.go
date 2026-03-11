package cmd

import (
	"fmt"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/restclient"
)

// RunAdapterInfo executes the adapter-info subcommand.
// It queries PARKING_FEE_SERVICE for adapter metadata.
func RunAdapterInfo(args []string, serviceURL string) error {
	operatorID, err := requireFlag(args, "operator-id")
	if err != nil {
		return err
	}

	client := restclient.New(serviceURL)
	path := fmt.Sprintf("/operators/%s/adapter", operatorID)
	body, err := client.Get(path)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(body)
}
