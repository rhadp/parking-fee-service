package cmd

import (
	"flag"
	"fmt"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/restclient"
)

func runAdapterInfo(args []string) error {
	fs := flag.NewFlagSet("adapter-info", flag.ContinueOnError)
	operatorID := fs.String("operator-id", "", "Operator ID (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *operatorID == "" {
		return fmt.Errorf("usage: parking-app-cli adapter-info --operator-id=<id>\n  --operator-id is required")
	}

	baseURL := config.ParkingFeeServiceURL()
	url := fmt.Sprintf("%s/operators/%s/adapter", baseURL, *operatorID)

	client := restclient.New()
	body, err := client.Get(url)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(body)
}
