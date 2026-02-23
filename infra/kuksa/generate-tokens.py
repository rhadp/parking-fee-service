#!/usr/bin/env python3
"""Generate JWT tokens for Kuksa Databroker service authorization.

This script generates RS256-signed JWT tokens for each RHIVOS safety-partition
service. Tokens are written as individual files to infra/kuksa/tokens/.

Prerequisites:
  - Python 3.8+
  - PyJWT library: pip install PyJWT[crypto]
  - RSA key pair at infra/kuksa/keys/jwt.key and jwt.pub.pem

Usage:
  python3 infra/kuksa/generate-tokens.py

  Or to regenerate keys and tokens:
    openssl genrsa -out infra/kuksa/keys/jwt.key 2048
    openssl rsa -in infra/kuksa/keys/jwt.key -pubout -out infra/kuksa/keys/jwt.pub.pem
    python3 infra/kuksa/generate-tokens.py
"""

import json
import os
import sys

try:
    import jwt
except ImportError:
    print("Error: PyJWT library not found. Install with: pip install PyJWT[crypto]", file=sys.stderr)
    sys.exit(1)

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
KEY_PATH = os.path.join(SCRIPT_DIR, "keys", "jwt.key")
TOKENS_DIR = os.path.join(SCRIPT_DIR, "tokens")

# Service token definitions
# Each service gets a JWT with specific VSS signal permissions.
# Permissions: "r" = read, "w" = write, "rw" = read+write
# Wildcards supported: "Vehicle.*" matches all Vehicle signals
SERVICES = {
    "locking-service": {
        "sub": "locking-service",
        "iss": "sdv-parking-demo",
        "iat": 1700000000,
        "exp": 4102444800,  # 2099-12-31
        "kuksa-vss": {
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": "rw",
            "Vehicle.Command.Door.Response": "rw",
            "Vehicle.Command.Door.Lock": "r",
            "Vehicle.Speed": "r",
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": "r",
        },
    },
    "cloud-gateway-client": {
        "sub": "cloud-gateway-client",
        "iss": "sdv-parking-demo",
        "iat": 1700000000,
        "exp": 4102444800,
        "kuksa-vss": {
            "Vehicle.Command.Door.Lock": "rw",
            "Vehicle.Command.Door.Response": "r",
            "Vehicle.Cabin.Door.Row1.DriverSide.IsLocked": "r",
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": "r",
            "Vehicle.CurrentLocation.Latitude": "r",
            "Vehicle.CurrentLocation.Longitude": "r",
            "Vehicle.Speed": "r",
        },
    },
    "location-sensor": {
        "sub": "location-sensor",
        "iss": "sdv-parking-demo",
        "iat": 1700000000,
        "exp": 4102444800,
        "kuksa-vss": {
            "Vehicle.CurrentLocation.Latitude": "rw",
            "Vehicle.CurrentLocation.Longitude": "rw",
        },
    },
    "speed-sensor": {
        "sub": "speed-sensor",
        "iss": "sdv-parking-demo",
        "iat": 1700000000,
        "exp": 4102444800,
        "kuksa-vss": {
            "Vehicle.Speed": "rw",
        },
    },
    "door-sensor": {
        "sub": "door-sensor",
        "iss": "sdv-parking-demo",
        "iat": 1700000000,
        "exp": 4102444800,
        "kuksa-vss": {
            "Vehicle.Cabin.Door.Row1.DriverSide.IsOpen": "rw",
        },
    },
    "admin": {
        "sub": "admin",
        "iss": "sdv-parking-demo",
        "iat": 1700000000,
        "exp": 4102444800,
        "kuksa-vss": {
            "Vehicle.*": "rw",
        },
    },
}


def main():
    if not os.path.exists(KEY_PATH):
        print(f"Error: Private key not found at {KEY_PATH}", file=sys.stderr)
        print("Generate keys with:", file=sys.stderr)
        print(f"  openssl genrsa -out {KEY_PATH} 2048", file=sys.stderr)
        print(
            f"  openssl rsa -in {KEY_PATH} -pubout -out {os.path.join(SCRIPT_DIR, 'keys', 'jwt.pub.pem')}",
            file=sys.stderr,
        )
        sys.exit(1)

    with open(KEY_PATH, "r") as f:
        private_key = f.read()

    os.makedirs(TOKENS_DIR, exist_ok=True)

    for service_name, claims in SERVICES.items():
        token = jwt.encode(claims, private_key, algorithm="RS256")
        token_file = os.path.join(TOKENS_DIR, f"{service_name}.token")
        with open(token_file, "w") as f:
            f.write(token)
        print(f"Generated: {token_file}")

    # Write summary JSON for documentation
    summary = {
        name: {
            "file": f"tokens/{name}.token",
            "permissions": claims["kuksa-vss"],
        }
        for name, claims in SERVICES.items()
    }
    summary_path = os.path.join(SCRIPT_DIR, "tokens.json")
    with open(summary_path, "w") as f:
        json.dump(summary, f, indent=2)
        f.write("\n")
    print(f"\nToken summary: {summary_path}")


if __name__ == "__main__":
    main()
