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

Wasted 1+ hours on configuring GitHub MCP across all tools

Restarted the development on `main`. Created a dedicated AGENT.md, to guide Claude Code, Cursor and Kiro consistently.
Credits at start: 449

Restarted because Gitflow was broken

Stuck at Task 6. Container build is "wrong"
Credits: 601

Many manual interventions to get the agent "unstuck" ...

---
read the requirements.md, design.md and task.md documents in .kiro/specs/project-foundation. all tasks are implemented. make sure you understand the specification. next, analyze the code etc and make sure that what was required, designed and defined to be implemented is actually implemented, works as intended by the specifications. excute any test if you need to verify code correctness. also point out any potential issues and suggest areas to be improved. remember, this is a proof-of-concept, not a production system, therfor it is OK to simplify aspects of the implementation, unless it makes the implementation unreasonalbe or unrealistic. create a markdown document with your findings and recommendations in docs/reviews/{review}.

---
06.2.2026
debugging:
credits 728

---
Let me check the specs directory to see what features are available and their current state.

Based on the design documents and the project structure, I can see the dependency chain. Let me also check the project-foundation spec to understand the overall implementation order:

Now let me check the tasks.md files for the other specs to see their completion status:

Now I have a clear picture. Based on the dependency analysis from the design documents and task completion status:

**Implementation Order After LOCKING_SERVICE:**

Looking at the architecture and dependencies:

1. **LOCKING_SERVICE** (partially complete - tasks 1-5 done, 6-14 remaining)
   - Core service that handles door lock/unlock
   - Depends on: DATA_BROKER (external Eclipse Kuksa)
   - Required by: CLOUD_GATEWAY_CLIENT, PARKING_OPERATOR_ADAPTOR

2. **CLOUD_GATEWAY_CLIENT** (not started)
   - Bridges cloud commands to LOCKING_SERVICE
   - Depends on: LOCKING_SERVICE, DATA_BROKER
   - Required by: COMPANION_APP (for remote lock/unlock)

3. **UPDATE_SERVICE** (not started)
   - Manages container lifecycle for adapters
   - Depends on: External registry, podman
   - Required by: PARKING_APP (to install adapters)

4. **PARKING_OPERATOR_ADAPTOR** (not started)
   - Manages parking sessions based on lock state
   - Depends on: DATA_BROKER (subscribes to IsLocked), PARKING_FEE_SERVICE (zone lookup)
   - Runs as container managed by UPDATE_SERVICE

**Recommended next spec: CLOUD_GATEWAY_CLIENT**

This is the logical next step because:
- It directly depends on LOCKING_SERVICE (which you're finishing)
- It enables the COMPANION_APP to send remote lock/unlock commands
- It's part of the safety partition (ASIL-B) like LOCKING_SERVICE
- It completes the cloud-to-vehicle command path

Alternatively, **UPDATE_SERVICE** could be next if you want to enable the adapter installation flow first, but CLOUD_GATEWAY_CLIENT has a tighter coupling to LOCKING_SERVICE.


CLOUD_GATEWAY -> PARKING_FEE_SERVICE
CLOUD_GATEWAY_CLIENT → UPDATE_SERVICE → PARKING_OPERATOR_ADAPTOR → PARKING_APP → COMPANION_APP


---
Let me check the specs directory and the current implementation state to determine what should be implemented next.

Based on my analysis of the tasks.md files, here's the implementation status:

| Spec | Status |
|------|--------|
| project-foundation | ✅ Complete |
| locking-service | ✅ Complete |
| parking-fee-service | ✅ Complete |
| cloud-gateway | ✅ Complete |
| cloud-gateway-client | 🔄 In Progress (tasks 1-13 done, tasks 14-20 remaining) |
| parking-operator-adaptor | ❌ Not Started |
| update-service | ❌ Not Started |
| parking-app | ❌ Not Started |
| companion_app | ❌ Not Started |

The next spec to implement is **cloud-gateway-client** — it's partially complete and needs to finish tasks 14-20 which cover:
- Telemetry subscription and publishing with offline buffering
- Structured logging
- Main service startup and graceful shutdown
- Integration testing

After that, the logical order would be:
1. **update-service** — manages container lifecycle for parking adapters
2. **parking-operator-adaptor** — depends on update-service for container management
3. **parking-app** — the Android IVI app that ties everything together
4. **companion_app** — the mobile companion app

Would you like me to continue implementing the cloud-gateway-client spec starting from task 14?


---
New spec:

In order to test the overal system locally developing both Android app, I want to have a command-line app for each. these apps should:
- allow me to send commands / events like the "real" apps. example: for the COMPANION_APP, send a lock/unlock command and also display the current status of the car. 
- for the PARKING_APP it would be: "fake" a location, list available parking, select parking, start/stop parking etc.
- The apps should be simple golang CLIs that basically are a REPL, like Claude code is.
- The apps should use the same messaging interfaces as the android apps would
