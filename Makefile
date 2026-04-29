.PHONY: check test lint proto

# Run lint + all tests
check: lint test

# Run all tests
test:
	cd rhivos && cargo test -p mock-sensors
	cd mock/parking-operator && go test -v ./...
	cd tests/mock-apps && go test -v ./...

# Run linters
lint:
	cd rhivos && cargo clippy -p mock-sensors -- -D warnings
	go vet ./mock/parking-operator/...
	go vet ./mock/companion-app-cli/...
	go vet ./mock/parking-app-cli/...
	go vet ./tests/mock-apps/...

# Regenerate protobuf Go stubs
proto:
	protoc --go_out=gen --go_opt=paths=source_relative \
		--go-grpc_out=gen --go-grpc_opt=paths=source_relative \
		-I proto \
		proto/update/update_service.proto \
		proto/adapter/adapter_service.proto \
		proto/kuksa/val.proto
