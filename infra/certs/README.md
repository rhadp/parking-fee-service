# Development TLS Certificates

This directory contains self-signed TLS certificates for local development.

## ⚠️ WARNING

These certificates are for **DEVELOPMENT ONLY**. Do NOT use in production.

## Directory Structure

```
certs/
├── ca/
│   ├── ca.crt          # Root CA certificate (share with clients)
│   └── ca.key          # Root CA private key (KEEP SECURE)
├── server/
│   ├── server.crt      # Server certificate
│   └── server.key      # Server private key
└── client/
    ├── client.crt      # Client certificate (for mTLS)
    └── client.key      # Client private key
```

## Generating Certificates

Run the generation script:

```bash
./scripts/generate-certs.sh
```

Or use make:

```bash
make certs
```

## Using Certificates

### Server Configuration

```yaml
tls:
  cert_file: infra/certs/server/server.crt
  key_file: infra/certs/server/server.key
  ca_file: infra/certs/ca/ca.crt
```

### Client Configuration

```yaml
tls:
  cert_file: infra/certs/client/client.crt
  key_file: infra/certs/client/client.key
  ca_file: infra/certs/ca/ca.crt
```

### Disabling TLS Verification (Development Only)

Set environment variable:

```bash
export SDV_TLS_SKIP_VERIFY=true
```

## Certificate Details

- **Key Size**: 4096 bits RSA
- **CA Validity**: 10 years
- **Certificate Validity**: 1 year
- **Hash Algorithm**: SHA-256

## Regenerating Certificates

To regenerate all certificates:

```bash
./scripts/generate-certs.sh clean
./scripts/generate-certs.sh
```

## Subject Alternative Names (SANs)

Server certificates include SANs for:
- localhost
- All service names (kuksa-databroker, mosquitto, etc.)
- 127.0.0.1 and ::1
