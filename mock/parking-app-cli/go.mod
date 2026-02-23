module github.com/rhadp/parking-fee-service/mock/parking-app-cli

go 1.23

require (
	github.com/rhadp/parking-fee-service/gen/go v0.0.0-00010101000000-000000000000
	github.com/spf13/cobra v1.9.1
	google.golang.org/grpc v1.72.1
)

require (
	github.com/inconshreveable/mousetrap v1.1.0 // indirect
	github.com/spf13/pflag v1.0.6 // indirect
	golang.org/x/net v0.35.0 // indirect
	golang.org/x/sys v0.30.0 // indirect
	golang.org/x/text v0.22.0 // indirect
	google.golang.org/genproto/googleapis/rpc v0.0.0-20250218202821-56aae31c358a // indirect
	google.golang.org/protobuf v1.36.6 // indirect
)

replace github.com/rhadp/parking-fee-service/gen/go => ../../gen/go
