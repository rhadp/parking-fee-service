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
-> use this as a starting point: <https://gitlab.com/CentOS/automotive/container-images/eclipse-kuksa>

The PRD mentions "pre-signed adapters (simplified trust chain)" - how simplified? Should we skip signature verification entirely for the demo, or implement basic checksum validation?
-> creating the container in the CI/CD pipeline should create a manifest that allows to alidate against

For local development infrastructure - do you already have Podman and an MQTT broker set up, or should the spec include setup instructions for these?
-> podman is available, MQTT should use a container

What's your timeline for the demo? This helps prioritize which scenarios (Happy Path, Adapter Already Installed, Error Handling) to focus on first.
-> everything is needed for the demo

---

Credits after the spec phase: 241

---

use claude code to be a critic of the design after completing the docs.

"read the prd.md document and make sure you understand it. Then traverse the requirements, design and task documents in folder .kiro/. and analyze and understand them as well. The documents in .kiro/ will guide the implementation of the system. now make sure they are complete,
  consisten etc. point out any issues, gaps or make sugestions what COULD be changed or improved. create a markdown document in .kiro with your findings."

Credits after review phase: 376

---
use claude code to verify component cohesion:

"The documents in .kiro/ will guide the implementation of the system. Read the requirements, design and task documents in folder .kiro/. and analyze and understand them. Now verify the component's communicational cohesion. look for inconsistencies in the messaging between the components. Create a markdown document in .kiro with your findings. "

"read file .kiro/communication-cohesion-findings.md. It conatins a list of critical issues with the various requirements, design and task documents. fix them. update all affected documents"

Credits after review: 442

---
05.02.2026:

Restarted the development on `main`. Created a dedicated AGENT.md, to guide Claude Code, Cursor and Kiro consistently.


