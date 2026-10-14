# CodeMind

> **AI codebase investigation agent for connected software intelligence, powered by FalkorDB.**

CodeMind helps a developer answer:

> **“What could this change affect?”**

It analyzes a developer-supplied repository, builds a language-agnostic software graph in FalkorDB, traces connected code and test relationships, and uses deterministic Rust intelligence plus a bounded LLM agent to explain impact, risk, test evidence, and graph paths.

## Supported Languages

The MVP supports repositories containing:

- Rust
- JavaScript
- TypeScript
- Python

The graph model is language-agnostic. Language-specific adapters translate supported syntax and semantics into the shared canonical model. CodeMind is **not coupled to Nizaam Core, a control-plane repository, or a predefined demo repository**.

Unsupported syntax and unresolved relationships are reported honestly rather than converted into fabricated graph facts.

---

## Architecture

```text
Browser
   ↓
Next.js + server-side Agent Runtime
   ↓
Semantic tools
   ↓
gRPC / Protobuf
   ↓
Rust codemind-engine
   │
   ├── CoIE — Code Investigation Engine
   │      └── source analysis + canonical Code Graph
   │
   ├── TIE — Test Investigation Engine
   │      └── test discovery + test evidence
   │
   └── ChIE — Change Investigation Engine
          └── impact + risk + test recommendations + evidence
   ↓
Graph Storage Port
   ↓
FalkorDB
```

There is one Next.js application, one Rust service, and one graph database. CoIE, TIE, and ChIE are modules inside the Rust service, not separate services.

React, Next.js Route Handlers, the Agent Runtime, and the LLM never access FalkorDB directly.

### Three primary processes

1. FalkorDB, normally through Docker Compose.
2. The Rust `codemind-engine` service.
3. The Next.js web application containing the server-side Agent Runtime.

There is no Express backend and no separately deployed Agent Runtime.

---

## Investigation Flow

A typical investigation is:

```text
Developer
   │
   │ “What could this change affect?”
   ▼
Agent Runtime
   ↓
semantic tool selection
   ↓
Rust gRPC boundary
   ↓
CoIE → TIE → ChIE
   ↓
real FalkorDB graph traversal
   ↓
impact + risk + test evidence + paths
   ↓
canonical result
   ├── structured JSON → UI
   └── bounded projection → flat → TOON → LLM
```

FalkorDB performs real relationship traversal. It is not a decorative visualization database.

---

## Graph Model

The graph represents software concepts rather than language-specific syntax.

Typical canonical entities include:

```text
Repository
Snapshot
File
Module
Symbol
Function
Method
Class
Type
Test
TestSuite
ChangeSet
ChangeRecord
AnalysisRun
ImpactAnalysis
RiskAssessment
Evidence
ConversationSession
ConversationTurn
ReviewEpisode
ReviewFinding
SemanticMemory
ProcedureMemory
Handoff
EngineReference
```

Typical relationships include:

```text
CONTAINS
DEFINES
IMPORTS
EXPORTS
CALLS
REFERENCES
IMPLEMENTS
EXTENDS
TARGETS
EXERCISES
COVERS
AFFECTS
RECOMMENDS
SUPPORTED_BY
EVIDENCES
```

The canonical model is deliberately language-agnostic. The Rust, JavaScript/TypeScript, and Python adapters map language-specific constructs into these shared semantics.

Important graph queries and algorithms are documented in `docs/architecture.md`, `docs/implementation-plan.md`, and this README as the implementation becomes concrete. The actual Cypher used by the product must be documented at submission time.

---

## Engines

### CoIE

CoIE owns canonical code/source facts:

- repository snapshots
- files and modules
- declarations and symbols
- imports/exports
- supported resolvable calls/references
- source evidence
- Code Graph construction
- bounded code queries

### TIE

TIE owns test facts:

- static test discovery
- test/test-suite entities
- supported test-to-code mappings
- test evidence
- Test Graph construction

Static discovery does not execute arbitrary repository tests. `TARGETS` is only asserted when the mapping is supported. Runtime coverage semantics are not fabricated.

### ChIE

ChIE owns change-derived conclusions:

- ChangeSet
- AnalysisRun
- direct/indirect impact
- qualitative risk
- test recommendations
- evidence
- Change Graph information

ChIE does not duplicate canonical CoIE or TIE nodes.

The guaranteed hackathon engine path is:

```text
CoIE → TIE → ChIE
```

---

## Semantic Tools

The MVP exposes 11 semantic tools:

1. `get_repository_context`
2. `search_code`
3. `get_symbol`
4. `query_code_relationships`
5. `get_historical_changes`
6. `get_test_intelligence`
7. `analyze_change`
8. `get_impact`
9. `get_risk`
10. `get_test_impact`
11. `get_evidence`

`simulate_change` is intentionally deferred.

The LLM cannot issue arbitrary Cypher, execute arbitrary shell commands, invoke private Rust functions, manufacture authorization context, or directly persist trusted memory.

---

## Canonical JSON and TOON Result Pipeline

The canonical engine result remains structured data.

```text
Rust Type B canonical result
        ↓
JSON payload in typed Protobuf response
        ↓
parse + validate in JavaScript
        ↓
bounded LLM projection
        ↓
flat
        ↓
@toon-format/toon
        ↓
TOON text as LLM tool-result content
```

The UI receives structured JSON/UI DTOs.

Successful LLM-facing Type B results must use the shared `flat → @toon-format/toon` pipeline. There is no silent JSON fallback and no silent direct-TOON bypass.

The flattening configuration must preserve arrays and array records and must be reversible for supported structures. Collision-prone keys must be handled explicitly.

The project also measures compact JSON, direct TOON, flattened JSON, and flattened TOON on representative CodeMind payloads. These are measurements, not a universal token-saving claim.

---

## Testing Strategy

Testing is continuous throughout the four implementation days.

### Engine Level 1 — Unit / Component

Deterministic Rust tests cover:

- canonical identity and normalization
- language extraction
- relationship resolution
- graph model semantics
- TIE discovery
- ChIE impact classification
- risk factors
- evidence
- result/error semantics
- lifecycle transitions
- memory/handoff state transitions

### Engine Level 2 — Integration

Real subsystem boundaries are tested with:

- FalkorDB
- repository snapshots
- graph reads/writes
- idempotent writes
- workspace isolation
- language adapters
- Type A / Type B contracts
- gRPC
- memory persistence
- handoff persistence

At least one integration test must prove that a real persisted graph relationship materially changes an investigation result.

### Engine Level 3 — Pipeline / System

The complete engine route is tested:

```text
repository
  ↓
source snapshot
  ↓
CoIE
  ↓
TIE
  ↓
ChIE
  ↓
canonical result
  ↓
gRPC
```

A green unit suite alone is not sufficient proof that the engines work.

### JavaScript semantic-tool tests

The JavaScript layer must explicitly prove that it receives a valid JSON-derived canonical result before LLM encoding:

- gRPC response received
- JSON payload parses
- schema validation succeeds
- tool receives structured data
- UI projection remains structured JSON
- malformed payload fails explicitly
- `flat`/`unflatten` is reversible for supported shapes
- arrays remain meaningful records
- TOON is the final LLM-facing representation
- successful Type B results do not silently fall back to JSON

### Browser and final E2E

Playwright covers application E2E. Day 4 additionally performs full manual E2E testing and TestSprite verification.

TestSprite is an additional application-level verification layer, not a replacement for deterministic Rust tests, integration tests, or semantic-tool contract tests.

---

## Four-Day Implementation

### Day 1 — Foundation

Prove the real repository-analysis and graph vertical slice:

```text
repository
 → language detection
 → Rust/JS/TS/Python adapters
 → canonical Code/Test Graph
 → FalkorDB
 → bounded query
 → gRPC
 → Next.js
```

### Day 2 — Track 01

Prove connected-data reasoning:

```text
semantic tools
 → CoIE
 → TIE
 → ChIE
 → multi-hop graph reasoning
 → impact/risk/tests/evidence
 → LLM explanation
```

### Day 3 — Track 02 + UI

Prove persistent coordination:

```text
conversation
 → review episode/finding
 → semantic/procedure memory
 → persistent handoff
 → fresh session
 → current-evidence validation
```

Then complete the functional chat-first UI.

### Day 4 — Freeze and Verify

No new major features. Perform:

- clean-start verification
- all unit tests
- all integration tests
- all three engine levels
- language conformance
- JavaScript JSON-output/tool contract tests
- `flat → TOON` tests
- Playwright E2E
- Rust/JavaScript/TypeScript/Python E2E
- Track 01 and Track 02 E2E
- failure/recovery/reconnect tests
- manual end-to-end testing
- TestSprite testing
- documentation completion
- demo rehearsal
- submission verification

---

## Repository Structure

```text
codemind/
├── AGENTS.md
├── README.md
├── package.json
├── pnpm-workspace.yaml
├── playwright.config.ts
├── .env.example
├── .gitignore
├── docker-compose.yml
│
├── apps/
│   └── web/
│       ├── package.json
│       ├── next.config.ts
│       ├── tsconfig.json
│       ├── vitest.config.ts
│       └── src/
│           ├── app/
│           ├── components/
│           │   ├── ui/
│           │   ├── shell/
│           │   ├── chat/
│           │   ├── execution/
│           │   ├── analysis/
│           │   ├── graph/
│           │   ├── evidence/
│           │   ├── memory/
│           │   ├── repository/
│           │   ├── settings/
│           │   └── common/
│           ├── lib/
│           │   ├── shared/
│           │   └── server/
│           │       ├── agent/
│           │       ├── tools/
│           │       ├── workflows/
│           │       ├── conversations/
│           │       ├── memory/
│           │       ├── handoffs/
│           │       ├── operations/
│           │       ├── results/
│           │       ├── rust-client/
│           │       ├── auth/
│           │       └── config/
│           └── tests/
│               ├── e2e/
│               ├── language-adapters/
│               ├── graph-conformance/
│               └── memory/
│
├── proto/
│   └── v1/
│       ├── common.proto
│       ├── repository.proto
│       ├── intelligence.proto
│       ├── memory.proto
│       └── operation.proto
│
├── engine/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── src/
│   │   ├── transport/
│   │   ├── contract/
│   │   ├── source/
│   │   ├── engines/
│   │   │   ├── coie/
│   │   │   ├── tie/
│   │   │   └── chie/
│   │   ├── graph_storage/
│   │   │   └── falkordb/
│   │   ├── memory/
│   │   └── lifecycle/
│   └── tests/
│
├── docs/
│   ├── architecture.md
│   ├── implementation-plan.md
│   ├── testing.md
│   ├── design-system.md
│   ├── demo.md
│   └── design/
│
└── scripts/
    ├── setup.sh
    ├── generate-proto.sh
    ├── run-e2e.sh
    └── verify-clean-start.sh
```

There is deliberately **no root `fixtures/` directory**. Test repositories or source snippets needed for automated tests should be isolated temporary inputs and must not become a product dependency.

There are also no separate `graph-model.md`, `language-support.md`, or `graph-queries.md` documents. Their information belongs in the existing architecture, implementation, testing, demo, and README documentation.

---

## Security and Trust Boundaries

Repository content is untrusted input. This includes source, comments, README files, test names, reports, and logs.

Repository content cannot override CodeMind instructions, authorization, or tool policies.

CodeMind does not execute arbitrary repository code as part of static analysis.

The LLM cannot:

- access FalkorDB directly
- issue arbitrary Cypher
- execute arbitrary shell commands
- manufacture permissions
- directly persist trusted memory
- invent canonical graph facts

Authorization and repository/snapshot context come from trusted runtime state.

---

## Documentation

| Document | Responsibility |
|---|---|
| `AGENTS.md` | Non-negotiable implementation rules |
| `docs/architecture.md` | Frozen architecture and domain semantics |
| `docs/implementation-plan.md` | Four-day file-by-file implementation plan |
| `docs/testing.md` | Detailed testing strategy and acceptance checks |
| `docs/demo.md` | Demo story and submission proof |
| `docs/design-system.md` | UI design system |
| `docs/design/` | Approved visual references |

`AGENTS.md` remains a **single root file**. We do not create separate `AGENTS.md` files inside `apps/web` or `engine`, because one project-wide rule set avoids contradictory implementation rules between the two sides of the architecture.

---

## Hackathon Tracks

Current targets:

- **Track 01 — Agents That Act on Connected Data**
- **Track 02 — Agent Memory and Coordination**

Track 03 is outside the current MVP scope.

---

## AI-Assistance Disclosure

CodeMind's architecture, planning, implementation guidance, and development may use AI coding assistance.

The final hackathon submission must include the disclosure required by the event rules and accurately represent the use of AI assistance.

---

## Status

CodeMind is implemented during the official Graph Hacks implementation window.

This README describes the intended product and architecture. A feature is considered complete only after its required tests and acceptance gate pass. Planned behavior is never treated as implemented behavior.
