package cmd

import (
	"flag"
	"fmt"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/config"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/restclient"
)

func runLookup(args []string) error {
	fs := flag.NewFlagSet("lookup", flag.ContinueOnError)
	lat := fs.String("lat", "", "Latitude value (required)")
	lon := fs.String("lon", "", "Longitude value (required)")

	if err := fs.Parse(args); err != nil {
		return err
	}

	if *lat == "" || *lon == "" {
		return fmt.Errorf("usage: parking-app-cli lookup --lat=<lat> --lon=<lon>\n  both --lat and --lon are required")
	}

	baseURL := config.ParkingFeeServiceURL()
	url := fmt.Sprintf("%s/operators?lat=%s&lon=%s", baseURL, *lat, *lon)

	client := restclient.New()
	body, err := client.Get(url)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(body)
}
