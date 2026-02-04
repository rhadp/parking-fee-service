# Product Overview

Parking Fee Service - a Software-Defined Vehicle (SDV) demo system showcasing mixed-criticality architecture.

## Purpose

Demonstrates automatic parking fee payment through vehicle-to-cloud integration:
- Android IVI parking app communicates with ASIL-B door locking service on RHIVOS
- Dynamic parking operator adapters download on-demand based on vehicle location
- Cross-domain communication between QM-level Android app and safety-relevant locking system

## Core User Flow

1. Vehicle parks → location detected → appropriate parking adapter downloads
2. User locks vehicle → parking session starts automatically
3. User unlocks vehicle → session ends, payment processed
4. Unused adapters offload after 24 hours

## Key Components

- **PARKING_APP**: Android Automotive OS IVI application
- **LOCKING_SERVICE**: ASIL-B door lock service (RHIVOS safety partition)
- **DATA_BROKER**: Eclipse Kuksa VSS-compliant signal broker
- **PARKING_OPERATOR_ADAPTOR**: Dynamic containerized adapters (RHIVOS QM partition)
- **UPDATE_SERVICE**: Container lifecycle management
- **CLOUD_GATEWAY**: Vehicle-to-cloud MQTT connectivity
- **COMPANION_APP**: Mobile app for remote lock/unlock

## Demo Scope

- Mock payment processing (no real transactions)
- Simulated location/sensors
- Pre-signed adapters (simplified trust chain)
- Minimal UI for Android apps
