# Development protocol

29.01.2026
Started with the PRD.md

04.02.2026
PRD.md "good enough" to move to Phase 1

---
In Kiro, with Opus 4.5:
Credits at start: 120

"Read and analyze PRD.md. Before you start to create a new specm aks clarifiying questions if needed."

Questions:

Should I create one large spec covering the entire system, or would you prefer separate specs per component/phase? Given the complexity, I'd recommend starting with a single component.
-> create a single spec for the components and design decissions needed by all components, e.g. project structure, interafce definitions etc
-> after that, create a separate spec for each component so that they could be implemented in parallel.

For the DATA_BROKER - the PRD mentions using Eclipse Kuksa Databroker. Should we integrate the existing Kuksa project, or implement a simplified mock version for the demo?
-> intgrate the existing Kuksa project.
-> use this as a starting point: https://gitlab.com/CentOS/automotive/container-images/eclipse-kuksa

The PRD mentions "pre-signed adapters (simplified trust chain)" - how simplified? Should we skip signature verification entirely for the demo, or implement basic checksum validation?
-> creating the container in the CI/CD pipeline should create a manifest that allows to alidate against

For local development infrastructure - do you already have Podman and an MQTT broker set up, or should the spec include setup instructions for these?
-> podman is available, MQTT should use a container

What's your timeline for the demo? This helps prioritize which scenarios (Happy Path, Adapter Already Installed, Error Handling) to focus on first.
-> everything is needed for the demo

---
