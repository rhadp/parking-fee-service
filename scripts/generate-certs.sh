#!/usr/bin/env bash
# Copyright 2024 SDV Parking Demo System
# TLS Certificate generation script for local development
#
# This script generates self-signed certificates for:
# - Root CA (Certificate Authority)
# - Server certificates for gRPC/MQTT services
# - Client certificates for mutual TLS authentication
#
# Requirements: 5.5
#
# WARNING: These certificates are for DEVELOPMENT ONLY.
# Do NOT use in production environments.

set -euo pipefail

# Script directory and project root
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
CERTS_DIR="${PROJECT_ROOT}/infra/certs"

# Certificate configuration
CA_DAYS=3650          # CA valid for 10 years
CERT_DAYS=365         # Certificates valid for 1 year
KEY_SIZE=4096         # RSA key size
COUNTRY="US"
STATE="California"
LOCALITY="San Francisco"
ORGANIZATION="SDV Parking Demo"
ORG_UNIT="Development"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" >&2
}

# Check if openssl is available
check_prerequisites() {
    if ! command -v openssl &> /dev/null; then
        log_error "openssl is required but not found."
        log_error "Install hint: brew install openssl (macOS) or apt install openssl (Linux)"
        exit 1
    fi
    log_info "OpenSSL version: $(openssl version)"
}

# Create directory structure
create_directories() {
    log_info "Creating certificate directory structure..."
    
    mkdir -p "${CERTS_DIR}/ca"
    mkdir -p "${CERTS_DIR}/server"
    mkdir -p "${CERTS_DIR}/client"
    
    log_success "Directory structure created at ${CERTS_DIR}"
}

# Generate CA certificate
generate_ca() {
    log_info "Generating Root CA certificate..."
    
    local ca_key="${CERTS_DIR}/ca/ca.key"
    local ca_crt="${CERTS_DIR}/ca/ca.crt"
    local ca_cnf="${CERTS_DIR}/ca/ca.cnf"
    
    # Create CA configuration file
    cat > "${ca_cnf}" << EOF
[req]
default_bits = ${KEY_SIZE}
prompt = no
default_md = sha256
distinguished_name = dn
x509_extensions = v3_ca

[dn]
C = ${COUNTRY}
ST = ${STATE}
L = ${LOCALITY}
O = ${ORGANIZATION}
OU = ${ORG_UNIT}
CN = SDV Parking Demo Root CA

[v3_ca]
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid:always,issuer
basicConstraints = critical, CA:true, pathlen:1
keyUsage = critical, digitalSignature, cRLSign, keyCertSign
EOF

    # Generate CA private key
    openssl genrsa -out "${ca_key}" ${KEY_SIZE} 2>/dev/null
    chmod 600 "${ca_key}"
    
    # Generate CA certificate
    openssl req -x509 -new -nodes \
        -key "${ca_key}" \
        -sha256 \
        -days ${CA_DAYS} \
        -out "${ca_crt}" \
        -config "${ca_cnf}"
    
    log_success "CA certificate generated: ${ca_crt}"
    log_warn "CA private key at ${ca_key} - Keep this secure!"
}

# Generate server certificate
generate_server_cert() {
    log_info "Generating server certificate..."
    
    local server_key="${CERTS_DIR}/server/server.key"
    local server_csr="${CERTS_DIR}/server/server.csr"
    local server_crt="${CERTS_DIR}/server/server.crt"
    local server_cnf="${CERTS_DIR}/server/server.cnf"
    local server_ext="${CERTS_DIR}/server/server.ext"
    
    # Create server configuration file
    cat > "${server_cnf}" << EOF
[req]
default_bits = ${KEY_SIZE}
prompt = no
default_md = sha256
distinguished_name = dn
req_extensions = req_ext

[dn]
C = ${COUNTRY}
ST = ${STATE}
L = ${LOCALITY}
O = ${ORGANIZATION}
OU = ${ORG_UNIT}
CN = localhost

[req_ext]
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = kuksa-databroker
DNS.4 = mosquitto
DNS.5 = locking-service
DNS.6 = update-service
DNS.7 = parking-adaptor
DNS.8 = cloud-gateway
DNS.9 = parking-fee-service
DNS.10 = cloud-gateway-client
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

    # Create extensions file for signing
    cat > "${server_ext}" << EOF
authorityKeyIdentifier = keyid,issuer
basicConstraints = CA:FALSE
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = serverAuth
subjectAltName = @alt_names

[alt_names]
DNS.1 = localhost
DNS.2 = *.localhost
DNS.3 = kuksa-databroker
DNS.4 = mosquitto
DNS.5 = locking-service
DNS.6 = update-service
DNS.7 = parking-adaptor
DNS.8 = cloud-gateway
DNS.9 = parking-fee-service
DNS.10 = cloud-gateway-client
IP.1 = 127.0.0.1
IP.2 = ::1
EOF

    # Generate server private key
    openssl genrsa -out "${server_key}" ${KEY_SIZE} 2>/dev/null
    chmod 600 "${server_key}"
    
    # Generate certificate signing request
    openssl req -new \
        -key "${server_key}" \
        -out "${server_csr}" \
        -config "${server_cnf}"
    
    # Sign the certificate with CA
    openssl x509 -req \
        -in "${server_csr}" \
        -CA "${CERTS_DIR}/ca/ca.crt" \
        -CAkey "${CERTS_DIR}/ca/ca.key" \
        -CAcreateserial \
        -out "${server_crt}" \
        -days ${CERT_DAYS} \
        -sha256 \
        -extfile "${server_ext}"
    
    # Clean up CSR (not needed after signing)
    rm -f "${server_csr}"
    
    log_success "Server certificate generated: ${server_crt}"
}

# Generate client certificate
generate_client_cert() {
    log_info "Generating client certificate..."
    
    local client_key="${CERTS_DIR}/client/client.key"
    local client_csr="${CERTS_DIR}/client/client.csr"
    local client_crt="${CERTS_DIR}/client/client.crt"
    local client_cnf="${CERTS_DIR}/client/client.cnf"
    local client_ext="${CERTS_DIR}/client/client.ext"
    
    # Create client configuration file
    cat > "${client_cnf}" << EOF
[req]
default_bits = ${KEY_SIZE}
prompt = no
default_md = sha256
distinguished_name = dn

[dn]
C = ${COUNTRY}
ST = ${STATE}
L = ${LOCALITY}
O = ${ORGANIZATION}
OU = ${ORG_UNIT}
CN = SDV Client
EOF

    # Create extensions file for signing
    cat > "${client_ext}" << EOF
authorityKeyIdentifier = keyid,issuer
basicConstraints = CA:FALSE
keyUsage = critical, digitalSignature, keyEncipherment
extendedKeyUsage = clientAuth
EOF

    # Generate client private key
    openssl genrsa -out "${client_key}" ${KEY_SIZE} 2>/dev/null
    chmod 600 "${client_key}"
    
    # Generate certificate signing request
    openssl req -new \
        -key "${client_key}" \
        -out "${client_csr}" \
        -config "${client_cnf}"
    
    # Sign the certificate with CA
    openssl x509 -req \
        -in "${client_csr}" \
        -CA "${CERTS_DIR}/ca/ca.crt" \
        -CAkey "${CERTS_DIR}/ca/ca.key" \
        -CAcreateserial \
        -out "${client_crt}" \
        -days ${CERT_DAYS} \
        -sha256 \
        -extfile "${client_ext}"
    
    # Clean up CSR (not needed after signing)
    rm -f "${client_csr}"
    
    log_success "Client certificate generated: ${client_crt}"
}

# Verify certificates
verify_certificates() {
    log_info "Verifying certificate chain..."
    
    # Verify CA certificate
    openssl x509 -in "${CERTS_DIR}/ca/ca.crt" -noout -text > /dev/null 2>&1 || {
        log_error "CA certificate verification failed"
        return 1
    }
    
    # Verify server certificate against CA
    openssl verify -CAfile "${CERTS_DIR}/ca/ca.crt" "${CERTS_DIR}/server/server.crt" || {
        log_error "Server certificate verification failed"
        return 1
    }
    
    # Verify client certificate against CA
    openssl verify -CAfile "${CERTS_DIR}/ca/ca.crt" "${CERTS_DIR}/client/client.crt" || {
        log_error "Client certificate verification failed"
        return 1
    }
    
    log_success "All certificates verified successfully"
}

# Print certificate information
print_cert_info() {
    log_info "Certificate Information:"
    echo ""
    echo "CA Certificate:"
    openssl x509 -in "${CERTS_DIR}/ca/ca.crt" -noout -subject -dates | sed 's/^/  /'
    echo ""
    echo "Server Certificate:"
    openssl x509 -in "${CERTS_DIR}/server/server.crt" -noout -subject -dates | sed 's/^/  /'
    echo ""
    echo "Client Certificate:"
    openssl x509 -in "${CERTS_DIR}/client/client.crt" -noout -subject -dates | sed 's/^/  /'
    echo ""
}

# Create .gitignore for certs directory
create_gitignore() {
    cat > "${CERTS_DIR}/.gitignore" << 'EOF'
# Ignore all certificate files except .gitkeep and .gitignore
*
!.gitkeep
!.gitignore
!README.md
EOF
    log_info "Created .gitignore to prevent committing certificates"
}

# Create README for certs directory
create_readme() {
    cat > "${CERTS_DIR}/README.md" << 'EOF'
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
EOF
    log_info "Created README.md with certificate documentation"
}

# Clean certificates
clean() {
    log_info "Cleaning certificates..."
    
    rm -rf "${CERTS_DIR}/ca" 2>/dev/null || true
    rm -rf "${CERTS_DIR}/server" 2>/dev/null || true
    rm -rf "${CERTS_DIR}/client" 2>/dev/null || true
    rm -f "${CERTS_DIR}/README.md" 2>/dev/null || true
    
    log_success "Certificates cleaned"
}

# Print usage information
usage() {
    cat << EOF
Usage: $(basename "$0") [COMMAND]

Generate self-signed TLS certificates for SDV Parking Demo development.

Commands:
    generate    Generate all certificates (default)
    ca          Generate only CA certificate
    server      Generate only server certificate (requires CA)
    client      Generate only client certificate (requires CA)
    verify      Verify certificate chain
    info        Print certificate information
    clean       Remove all generated certificates
    help        Show this help message

Examples:
    $(basename "$0")           # Generate all certificates
    $(basename "$0") verify    # Verify certificate chain
    $(basename "$0") info      # Show certificate details
    $(basename "$0") clean     # Remove all certificates

Certificate Output:
    ${CERTS_DIR}/ca/           CA certificate and key
    ${CERTS_DIR}/server/       Server certificate and key
    ${CERTS_DIR}/client/       Client certificate and key

WARNING: These certificates are for DEVELOPMENT ONLY.
         Do NOT use in production environments.

EOF
}

# Main entry point
main() {
    local command="${1:-generate}"
    
    case "$command" in
        generate)
            check_prerequisites
            create_directories
            generate_ca
            generate_server_cert
            generate_client_cert
            verify_certificates
            print_cert_info
            create_gitignore
            create_readme
            log_success "All certificates generated successfully!"
            log_warn "Remember: These certificates are for DEVELOPMENT ONLY."
            ;;
        ca)
            check_prerequisites
            create_directories
            generate_ca
            ;;
        server)
            check_prerequisites
            if [[ ! -f "${CERTS_DIR}/ca/ca.crt" ]]; then
                log_error "CA certificate not found. Run '$(basename "$0") ca' first."
                exit 1
            fi
            generate_server_cert
            ;;
        client)
            check_prerequisites
            if [[ ! -f "${CERTS_DIR}/ca/ca.crt" ]]; then
                log_error "CA certificate not found. Run '$(basename "$0") ca' first."
                exit 1
            fi
            generate_client_cert
            ;;
        verify)
            verify_certificates
            ;;
        info)
            print_cert_info
            ;;
        clean)
            clean
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            log_error "Unknown command: $command"
            usage
            exit 1
            ;;
    esac
}

# Run main function
main "$@"
