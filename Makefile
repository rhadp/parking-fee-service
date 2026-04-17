.PHONY: build test lint check clean
.PHONY: build-rust build-go test-rust test-go lint-rust lint-go

MOCK_MODULES := mock/companion-app-cli mock/parking-app-cli mock/parking-operator

## build: compile all components
build: build-rust build-go

build-rust:
	cd rhivos && cargo build -p mock-sensors

build-go:
	$(foreach mod,$(MOCK_MODULES),cd $(CURDIR)/$(mod) && go build ./...;)

## test: run all unit tests
test: test-rust test-go

test-rust:
	cd rhivos && cargo test -p mock-sensors

test-go:
	$(foreach mod,$(MOCK_MODULES),cd $(CURDIR)/$(mod) && go test -v ./...;)

## lint: run linters
lint: lint-rust lint-go

lint-rust:
	cd rhivos && cargo clippy -p mock-sensors -- -D warnings

lint-go:
	$(foreach mod,$(MOCK_MODULES),cd $(CURDIR)/$(mod) && go vet ./...;)

## check: lint + test
check: lint test

## clean: remove build artifacts
clean:
	cd rhivos && cargo clean
	rm -rf tests/mock-apps/testdata/bin/
