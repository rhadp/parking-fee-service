module github.com/rhadp/parking-fee-service/mock/companion-app-cli

go 1.23

require (
	github.com/rhadp/parking-fee-service/gen/go v0.0.0-00010101000000-000000000000
	github.com/spf13/cobra v1.9.1
)

require (
	github.com/google/uuid v1.6.0 // indirect
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/spf13/pflag v1.0.6 // indirect
	google.golang.org/protobuf v1.36.6 // indirect
)

replace github.com/rhadp/parking-fee-service/gen/go => ../../gen/go
