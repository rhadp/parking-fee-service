package cmd

import (
	"flag"
	"fmt"

	"github.com/parking-fee-service/mock/companion-app-cli/internal/config"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/output"
	"github.com/parking-fee-service/mock/companion-app-cli/internal/restclient"
)

func runStatus(args []string) error {
	fs := flag.NewFlagSet("status", flag.ContinueOnError)
	vin := fs.String("vin", "", "Vehicle Identification Number (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *vin == "" {
		return fmt.Errorf("usage: companion-app-cli status --vin=<vin>\n  --vin is required")
	}

	token := config.BearerToken()

	baseURL := config.CloudGatewayURL()
	url := fmt.Sprintf("%s/vehicles/%s/status", baseURL, *vin)

	client := restclient.New(token)
	resp, err := client.Get(url)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(resp)
}
