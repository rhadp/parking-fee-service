# Agent-Fox Memory

## Gotchas

- **tracing_subscriber defaults to stdout.** The Rust `tracing_subscriber::fmt()`
  builder writes to stdout unless `.with_writer(std::io::stderr)` is added.
  All test harnesses in this project capture and assert on **stderr** for log
  output. Any new Rust service that uses tracing must write to stderr.

- **Skeleton tests vs implemented services.** When a spec replaces a skeleton
  binary with a full implementation (e.g. spec 04 for `cloud-gateway-client`),
  the skeleton verification tests in `tests/setup/build_verification_test.go`
  must be updated to exclude that binary. Skeleton tests expect the binary to
  print a version string and exit 0 with no arguments, which implemented
  services won't do.

- **`make check` does not run setup tests.** The `test-setup` target
  (`tests/setup/`) is separate from `make test`. Use `make test-setup` to run
  build verification tests.

- **Pre-existing lint failure in `tests/databroker`.** `go vet` fails on
  `tests/databroker/helpers_test.go` due to undefined `pb.DataType`. This is
  a known issue unrelated to other specs.
