package cmd

import (
	"fmt"

	"github.com/parking-fee-service/mock/parking-app-cli/internal/output"
	"github.com/parking-fee-service/mock/parking-app-cli/internal/restclient"
)

// RunLookup executes the lookup subcommand.
// It queries PARKING_FEE_SERVICE for operators near the given lat/lon.
func RunLookup(args []string, serviceURL string) error {
	lat, err := requireFlag(args, "lat")
	if err != nil {
		return err
	}
	lon, err := requireFlag(args, "lon")
	if err != nil {
		return err
	}

	client := restclient.New(serviceURL)
	path := fmt.Sprintf("/operators?lat=%s&lon=%s", lat, lon)
	body, err := client.Get(path)
	if err != nil {
		return err
	}

	return output.PrintRawJSON(body)
}
