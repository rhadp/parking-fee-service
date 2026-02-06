## Technology Stack

### Languages by Component

| Component | Language | Location |
|-----------|----------|----------|
| RHIVOS services | Rust | `rhivos/` |
| Android IVI app | Kotlin | `android/parking-app/` |
| Companion app | Flutter/Dart | `android/companion_app/` |
| Backend services | Go | `backend/` |

### Key Dependencies

- **Eclipse Kuksa Databroker**: VSS-compliant vehicle signal broker
- **Eclipse Mosquitto**: MQTT broker
- **Protocol Buffers**: Interface definitions (`proto/`)
- **Podman**: Container builds and local orchestration
- **Container Images**: Red Hat UBI10 based container images, or Centos AutoSD based

### External Dependencies

- **Google Artifact Registry**: OCI-compliant registry stores validated PARKING_OPERATOR_ADAPTOR images
