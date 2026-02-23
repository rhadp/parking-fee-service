package setup_test

import (
	"os"
	"path/filepath"
	"strings"
	"testing"
)

// TS-01-17: Go modules for backend services (01-REQ-4.1)
func TestGo_ModulesExist(t *testing.T) {
	root := repoRoot(t)

	services := map[string]string{
		"parking-fee-service": "github.com/rhadp/parking-fee-service/backend/parking-fee-service",
		"cloud-gateway":      "github.com/rhadp/parking-fee-service/backend/cloud-gateway",
	}

	for svc, expectedModule := range services {
		goMod := "backend/" + svc + "/go.mod"
		assertFileExists(t, root, goMod)
		assertFileContains(t, root, goMod, "module "+expectedModule)
	}
}

// TS-01-20: Go skeletons include generated proto code (01-REQ-4.4)
func TestGo_GeneratedProto(t *testing.T) {
	root := repoRoot(t)

	packages := []string{"commonpb", "updateservicepb", "parkingadaptorpb"}
	for _, pkg := range packages {
		dir := "gen/go/" + pkg
		assertDirExists(t, root, dir)
		files := globFiles(t, root, "gen/go/"+pkg+"/*.pb.go")
		if len(files) < 1 {
			t.Errorf("expected at least 1 .pb.go file in %s, found %d", dir, len(files))
		}
	}
}

// TS-01-23: Mock PARKING_APP CLI exists (01-REQ-5.1)
func TestMockCLI_ParkingAppExists(t *testing.T) {
	root := repoRoot(t)
	goMod := "mock/parking-app-cli/go.mod"
	assertFileExists(t, root, goMod)
	assertFileContains(t, root, goMod, "module github.com/rhadp/parking-fee-service/mock/parking-app-cli")
}

// TS-01-24: Mock COMPANION_APP CLI exists (01-REQ-5.2)
func TestMockCLI_CompanionAppExists(t *testing.T) {
	root := repoRoot(t)
	goMod := "mock/companion-app-cli/go.mod"
	assertFileExists(t, root, goMod)
	assertFileContains(t, root, goMod, "module github.com/rhadp/parking-fee-service/mock/companion-app-cli")
}

// TS-01-27: Mock CLI apps share proto definitions (01-REQ-5.5)
func TestMockCLI_ShareProtoImports(t *testing.T) {
	root := repoRoot(t)

	for _, app := range []string{"parking-app-cli", "companion-app-cli"} {
		appDir := filepath.Join(root, "mock", app)
		// Walk all .go source files and check for proto import
		found := false
		err := filepath.Walk(appDir, func(path string, info os.FileInfo, err error) error {
			if err != nil {
				return err
			}
			if !info.IsDir() && strings.HasSuffix(path, ".go") && !strings.HasSuffix(path, "_test.go") {
				data, readErr := os.ReadFile(path)
				if readErr != nil {
					return readErr
				}
				if strings.Contains(string(data), "github.com/rhadp/parking-fee-service/gen/go/") {
					found = true
				}
			}
			return nil
		})
		if err != nil {
			t.Errorf("error walking %s: %v", app, err)
			continue
		}
		if !found {
			t.Errorf("mock app %s does not import generated proto packages from gen/go/", app)
		}
	}
}
