# Erratum: GetVINForToken Signature (Spec 06)

## Divergence

The design document specifies `GetVINForToken` as a **method on Config**:

```go
func (c *Config) GetVINForToken(token string) (string, bool)
```

Called as: `cfg.GetVINForToken(token)`

## Actual Implementation

The implementation uses a **package-level function** in the `config` package:

```go
func GetVINForToken(cfg *model.Config, token string) (string, bool)
```

Called as: `config.GetVINForToken(cfg, token)`

## Rationale

The `Config` type is defined in the `model` package (`model.Config`), not in
the `config` package. Go does not allow defining methods on types from other
packages. Since the type definitions live in `model/` and the config loading
logic lives in `config/`, a package-level function is the idiomatic approach.

Adding a method to `model.Config` would couple the model package to the config
lookup logic, violating the separation of concerns established by the package
structure.

## Impact

- The `auth` middleware must call `config.GetVINForToken(cfg, token)` instead
  of `cfg.GetVINForToken(token)`.
- All existing tests already use the package-level function form.
- No functional difference in behavior.
