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
├── apps
│   └── web
│       ├── AGENTS.md
│       ├── README.md
│       ├── components.json
│       ├── docker-compose.yml
│       ├── eslint.config.mjs
│       ├── next-env.d.ts
│       ├── next.config.ts
│       ├── package.json
│       ├── playwright.config.ts
│       ├── pnpm-lock.yaml
│       ├── pnpm-workspace.yaml
│       ├── public
│       │   ├── file.svg
│       │   ├── globe.svg
│       │   ├── next.svg
│       │   ├── vercel.svg
│       │   └── window.svg
│       ├── src
│       │   ├── app
│       │   │   ├── analysis
│       │   │   │   └── [analysisId]
│       │   │   │       └── page.tsx
│       │   │   ├── api
│       │   │   │   ├── agent
│       │   │   │   │   └── route.ts
│       │   │   │   ├── analysis
│       │   │   │   │   ├── [analysisId]
│       │   │   │   │   │   └── route.ts
│       │   │   │   │   └── route.ts
│       │   │   │   ├── conversations
│       │   │   │   │   ├── [conversationId]
│       │   │   │   │   │   └── route.ts
│       │   │   │   │   ├── route.ts
│       │   │   │   │   └── search
│       │   │   │   │       └── route.ts
│       │   │   │   ├── memory
│       │   │   │   │   └── route.ts
│       │   │   │   ├── operations
│       │   │   │   │   └── [operationId]
│       │   │   │   │       ├── events
│       │   │   │   │       │   └── route.ts
│       │   │   │   │       └── route.ts
│       │   │   │   ├── relationships
│       │   │   │   │   └── route.ts
│       │   │   │   └── repositories
│       │   │   │       ├── [repositoryId]
│       │   │   │       │   └── route.ts
│       │   │   │       └── route.ts
│       │   │   ├── favicon.ico
│       │   │   ├── globals.css
│       │   │   ├── help
│       │   │   │   └── page.tsx
│       │   │   ├── layout.tsx
│       │   │   ├── page.tsx
│       │   │   ├── repositories
│       │   │   │   ├── [repositoryId]
│       │   │   │   │   └── page.tsx
│       │   │   │   ├── new
│       │   │   │   │   └── page.tsx
│       │   │   │   └── page.tsx
│       │   │   └── settings
│       │   │       └── page.tsx
│       │   ├── components
│       │   │   ├── analysis
│       │   │   │   ├── affected-files-panel.tsx
│       │   │   │   ├── impact-card.tsx
│       │   │   │   ├── recommendations-panel.tsx
│       │   │   │   ├── risk-card.tsx
│       │   │   │   ├── summary-panel.tsx
│       │   │   │   └── test-impact-panel.tsx
│       │   │   ├── chat
│       │   │   │   ├── chat-composer.tsx
│       │   │   │   ├── chat-panel.tsx
│       │   │   │   ├── conversation-list.tsx
│       │   │   │   ├── conversation-search-modal.tsx
│       │   │   │   ├── message-list.tsx
│       │   │   │   └── message-renderer.tsx
│       │   │   ├── common
│       │   │   │   ├── completeness-notice.tsx
│       │   │   │   ├── empty-state.tsx
│       │   │   │   ├── error-state.tsx
│       │   │   │   └── loading-state.tsx
│       │   │   ├── evidence
│       │   │   │   └── evidence-panel.tsx
│       │   │   ├── execution
│       │   │   │   ├── engine-accordion.tsx
│       │   │   │   ├── execution-timeline.tsx
│       │   │   │   └── operation-status.tsx
│       │   │   ├── graph
│       │   │   │   ├── dependency-graph.tsx
│       │   │   │   └── graph-path-panel.tsx
│       │   │   ├── memory
│       │   │   │   ├── handoff-status.tsx
│       │   │   │   └── memory-inspector.tsx
│       │   │   ├── repository
│       │   │   │   ├── repository-card.tsx
│       │   │   │   ├── repository-list.tsx
│       │   │   │   └── repository-setup-form.tsx
│       │   │   ├── settings
│       │   │   │   └── settings-navigation.tsx
│       │   │   ├── shell
│       │   │   │   ├── app-shell.tsx
│       │   │   │   └── sidebar.tsx
│       │   │   └── ui
│       │   │       ├── accordion.tsx
│       │   │       ├── alert.tsx
│       │   │       ├── badge.tsx
│       │   │       ├── button.tsx
│       │   │       ├── card.tsx
│       │   │       ├── chip.tsx
│       │   │       ├── code-block.tsx
│       │   │       ├── data-table.tsx
│       │   │       ├── icon-button.tsx
│       │   │       ├── input.tsx
│       │   │       ├── message.tsx
│       │   │       ├── metric-card.tsx
│       │   │       ├── modal.tsx
│       │   │       ├── result-list.tsx
│       │   │       ├── select.tsx
│       │   │       ├── separator.tsx
│       │   │       ├── sheet.tsx
│       │   │       ├── sidebar.tsx
│       │   │       ├── skeleton.tsx
│       │   │       ├── status-indicator.tsx
│       │   │       ├── tabs.tsx
│       │   │       ├── textarea.tsx
│       │   │       ├── timeline.tsx
│       │   │       └── tooltip.tsx
│       │   ├── hooks
│       │   │   ├── use-highlighted-code.ts
│       │   │   └── use-mobile.ts
│       │   ├── lib
│       │   │   ├── highlighter.tsx
│       │   │   ├── server
│       │   │   │   ├── agent
│       │   │   │   │   ├── prompts.ts
│       │   │   │   │   ├── provider.ts
│       │   │   │   │   ├── runtime.test.ts
│       │   │   │   │   ├── runtime.ts
│       │   │   │   │   └── tool-loop.ts
│       │   │   │   ├── auth
│       │   │   │   │   ├── authorize.ts
│       │   │   │   │   └── context.ts
│       │   │   │   ├── config
│       │   │   │   │   └── env.ts
│       │   │   │   ├── conversations
│       │   │   │   │   └── service.ts
│       │   │   │   ├── handoffs
│       │   │   │   │   ├── schema.ts
│       │   │   │   │   └── service.ts
│       │   │   │   ├── memory
│       │   │   │   │   ├── freshness.ts
│       │   │   │   │   ├── policy.ts
│       │   │   │   │   ├── retrieval.ts
│       │   │   │   │   └── service.ts
│       │   │   │   ├── operations
│       │   │   │   │   ├── events.ts
│       │   │   │   │   ├── service.ts
│       │   │   │   │   ├── sse.ts
│       │   │   │   │   └── state.ts
│       │   │   │   ├── results
│       │   │   │   │   ├── canonical-json.ts
│       │   │   │   │   ├── llm-encoding.test.ts
│       │   │   │   │   ├── llm-encoding.ts
│       │   │   │   │   ├── llm-projection.ts
│       │   │   │   │   ├── ui-projection.ts
│       │   │   │   │   └── validate.ts
│       │   │   │   ├── rust-client
│       │   │   │   │   ├── client.ts
│       │   │   │   │   ├── generated
│       │   │   │   │   │   ├── codemind
│       │   │   │   │   │   │   └── v1
│       │   │   │   │   │   │       ├── CanonicalResult.ts
│       │   │   │   │   │   │       ├── ChangeAnalysisQuery.ts
│       │   │   │   │   │   │       ├── Completeness.ts
│       │   │   │   │   │   │       ├── Correlation.ts
│       │   │   │   │   │   │       ├── Diagnostic.ts
│       │   │   │   │   │   │       ├── ErrorStatus.ts
│       │   │   │   │   │   │       ├── Freshness.ts
│       │   │   │   │   │   │       ├── Handoff.ts
│       │   │   │   │   │   │       ├── IntelligenceService.ts
│       │   │   │   │   │   │       ├── MemoryRecord.ts
│       │   │   │   │   │   │       ├── MemoryReference.ts
│       │   │   │   │   │   │       ├── MemoryResponse.ts
│       │   │   │   │   │   │       ├── MemoryService.ts
│       │   │   │   │   │   │       ├── Outcome.ts
│       │   │   │   │   │   │       ├── PersistHandoffRequest.ts
│       │   │   │   │   │   │       ├── PersistMemoryRequest.ts
│       │   │   │   │   │   │       ├── PersistReviewContextRequest.ts
│       │   │   │   │   │   │       ├── ProgressEvent.ts
│       │   │   │   │   │   │       ├── QueryResponse.ts
│       │   │   │   │   │   │       ├── RepositoryContext.ts
│       │   │   │   │   │   │       ├── RepositoryService.ts
│       │   │   │   │   │   │       ├── RepositorySnapshot.ts
│       │   │   │   │   │   │       ├── ResolveRepositoryRequest.ts
│       │   │   │   │   │   │       ├── ResolveRepositoryResponse.ts
│       │   │   │   │   │   │       ├── RetrieveHandoffRequest.ts
│       │   │   │   │   │   │       ├── RetrieveMemoryRequest.ts
│       │   │   │   │   │   │       ├── RetrieveMemoryResponse.ts
│       │   │   │   │   │   │       ├── RetrieveReviewContextRequest.ts
│       │   │   │   │   │   │       ├── ReviewContextRecord.ts
│       │   │   │   │   │   │       ├── SourceReference.ts
│       │   │   │   │   │   │       ├── SymbolRelationshipQuery.ts
│       │   │   │   │   │   │       └── TransitionHandoffRequest.ts
│       │   │   │   │   │   ├── common.ts
│       │   │   │   │   │   ├── intelligence.ts
│       │   │   │   │   │   ├── memory.ts
│       │   │   │   │   │   ├── operation.ts
│       │   │   │   │   │   └── repository.ts
│       │   │   │   │   └── progress-stream.ts
│       │   │   │   ├── tools
│       │   │   │   │   ├── catalog.ts
│       │   │   │   │   ├── executor.ts
│       │   │   │   │   ├── guardrails.test.ts
│       │   │   │   │   ├── guardrails.ts
│       │   │   │   │   ├── implementations
│       │   │   │   │   │   ├── analyze-change.ts
│       │   │   │   │   │   ├── get-evidence.ts
│       │   │   │   │   │   ├── get-historical-changes.ts
│       │   │   │   │   │   ├── get-impact.ts
│       │   │   │   │   │   ├── get-repository-context.ts
│       │   │   │   │   │   ├── get-risk.ts
│       │   │   │   │   │   ├── get-symbol.ts
│       │   │   │   │   │   ├── get-test-impact.ts
│       │   │   │   │   │   ├── get-test-intelligence.ts
│       │   │   │   │   │   ├── index.ts
│       │   │   │   │   │   ├── query-code-relationships.ts
│       │   │   │   │   │   └── search-code.ts
│       │   │   │   │   ├── registry.ts
│       │   │   │   │   └── schemas.ts
│       │   │   │   └── workflows
│       │   │   │       ├── finalize-review.ts
│       │   │   │       ├── investigate-change.ts
│       │   │   │       └── prepare-repository.ts
│       │   │   ├── shared
│       │   │   │   ├── status.ts
│       │   │   │   └── types.ts
│       │   │   └── utils.ts
│       │   └── tests
│       │       ├── e2e
│       │       │   ├── reconnect-recovery.spec.ts
│       │       │   └── track01.spec.ts
│       │       └── graph-conformance
│       │           ├── canonical-model.spec.ts
│       │           ├── relationship-semantics.spec.ts
│       │           └── snapshot-isolation.spec.ts
│       ├── tsconfig.json
│       ├── tsconfig.tsbuildinfo
│       └── vitest.config.ts
├── docs
│   ├── architecture.md
│   ├── demo.md
│   ├── design
│   │   ├── CodeMind AI Code Analysis Dashboard.png
│   │   ├── CodeMind AI Settings Dashboard.png
│   │   ├── CodeMind Agent Execution Dashboard.png
│   │   ├── CodeMind Control Plane Impact Dashboard.png
│   │   ├── CodeMind Conversation Search Interface.png
│   │   ├── CodeMind Dark Design System Board.png
│   │   ├── CodeMind Error States Dashboard.png
│   │   ├── CodeMind Help & Docs Dashboard.png
│   │   ├── CodeMind Pipeline Analysis Dashboard (1).png
│   │   ├── CodeMind Pipeline Analysis Dashboard.png
│   │   ├── CodeMind Repository Dashboard.png
│   │   ├── CodeMind Repository Setup Dashboard.png
│   │   ├── CodeMind Settings Dashboard.png
│   │   ├── CodeMind Zero-Risk Analysis Dashboard.png
│   │   ├── Developer AI Dashboard UI Mockup.png
│   │   └── README.md
│   ├── design-system.md
│   ├── implementation-plan.md
│   └── testing.md
├── engine
│   ├── Cargo.lock
│   ├── Cargo.toml
│   ├── build.rs
│   ├── cargo-check.log
│   ├── src
│   │   ├── config.rs
│   │   ├── contract
│   │   │   ├── context.rs
│   │   │   ├── error.rs
│   │   │   ├── events.rs
│   │   │   ├── intelligence.rs
│   │   │   ├── memory.rs
│   │   │   ├── mod.rs
│   │   │   ├── operation.rs
│   │   │   ├── repository.rs
│   │   │   └── result.rs
│   │   ├── engines
│   │   │   ├── chie
│   │   │   │   ├── change_set.rs
│   │   │   │   ├── engine.rs
│   │   │   │   ├── evidence.rs
│   │   │   │   ├── graph_builder.rs
│   │   │   │   ├── impact
│   │   │   │   │   ├── aggregation.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   └── traversal.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── model.rs
│   │   │   │   ├── queries.rs
│   │   │   │   ├── risk
│   │   │   │   │   ├── assessment.rs
│   │   │   │   │   ├── factors.rs
│   │   │   │   │   └── mod.rs
│   │   │   │   └── test_impact.rs
│   │   │   ├── coie
│   │   │   │   ├── engine.rs
│   │   │   │   ├── extraction
│   │   │   │   │   ├── evidence.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── normalize.rs
│   │   │   │   │   ├── resolve.rs
│   │   │   │   │   └── validate.rs
│   │   │   │   ├── graph_builder.rs
│   │   │   │   ├── languages
│   │   │   │   │   ├── javascript.rs
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── python.rs
│   │   │   │   │   ├── rust.rs
│   │   │   │   │   └── typescript.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── model.rs
│   │   │   │   └── queries
│   │   │   │       ├── history.rs
│   │   │   │       ├── mod.rs
│   │   │   │       ├── paths.rs
│   │   │   │       ├── relationships.rs
│   │   │   │       ├── search.rs
│   │   │   │       └── symbols.rs
│   │   │   ├── mod.rs
│   │   │   ├── pipeline.rs
│   │   │   └── tie
│   │   │       ├── discovery
│   │   │       │   ├── javascript_tests.rs
│   │   │       │   ├── mod.rs
│   │   │       │   ├── python_tests.rs
│   │   │       │   ├── rust_tests.rs
│   │   │       │   └── typescript_tests.rs
│   │   │       ├── engine.rs
│   │   │       ├── graph_builder.rs
│   │   │       ├── mapping
│   │   │       │   ├── evidence.rs
│   │   │       │   ├── mod.rs
│   │   │       │   └── target_mapping.rs
│   │   │       ├── mod.rs
│   │   │       ├── model.rs
│   │   │       └── queries
│   │   │           ├── mod.rs
│   │   │           └── tests.rs
│   │   ├── graph_storage
│   │   │   ├── falkordb
│   │   │   │   ├── client.rs
│   │   │   │   ├── mapping.rs
│   │   │   │   ├── mod.rs
│   │   │   │   ├── read.rs
│   │   │   │   ├── schema.rs
│   │   │   │   └── write.rs
│   │   │   ├── mod.rs
│   │   │   └── port.rs
│   │   ├── lib.rs
│   │   ├── lifecycle
│   │   │   ├── events.rs
│   │   │   ├── mod.rs
│   │   │   ├── operation.rs
│   │   │   ├── progress.rs
│   │   │   └── recovery.rs
│   │   ├── main.rs
│   │   ├── memory
│   │   │   ├── handoffs.rs
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── repository.rs
│   │   │   ├── scope.rs
│   │   │   ├── service.rs
│   │   │   └── validation.rs
│   │   ├── source
│   │   │   ├── identity.rs
│   │   │   ├── language.rs
│   │   │   ├── limits.rs
│   │   │   ├── manifest.rs
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs
│   │   │   ├── snapshot.rs
│   │   │   └── tree_sitter.rs
│   │   └── transport
│   │       ├── conversions.rs
│   │       ├── mod.rs
│   │       ├── progress.rs
│   │       └── server.rs
│   └── tests
│       ├── bounded_queries.rs
│       ├── canonical_graph.rs
│       ├── chie_impact.rs
│       ├── day1_boundaries.rs
│       ├── falkordb_integration.rs
│       ├── grpc_contract.rs
│       ├── handoff_lifecycle.rs
│       ├── language_adapters.rs
│       ├── level3_engines.rs
│       ├── level3_memory.rs
│       ├── level3_operation_grpc.rs
│       ├── level3_storage.rs
│       ├── level3_transport.rs
│       ├── memory_persistence.rs
│       ├── operation_recovery.rs
│       ├── support.rs
│       └── tie_discovery.rs
├── proto
│   └── v1
│       ├── common.proto
│       ├── intelligence.proto
│       ├── memory.proto
│       ├── operation.proto
│       └── repository.proto
└── scripts
    ├── generate-proto.sh
    ├── run-e2e.sh
    ├── run-level3.sh
    ├── setup.sh
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
| --- | --- |
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

## Beyond the Hackathon

The hackathon MVP is the first milestone, not the complete long-term vision for CodeMind. The immediate goal is to prove that connected graph reasoning, deterministic engine outputs, evidence-backed explanations, and persistent investigation context work together end to end. Future work will build on that foundation rather than replace it with an unconstrained LLM-only workflow.

Potential post-hackathon directions include:

1. **Deeper change and pull-request analysis.** Move beyond the MVP's symbol-first prospective-change workflow toward Git-diff and pull-request investigations, incremental graph updates, and richer historical-change reasoning.
2. **Broader code intelligence.** Extend language and framework adapters while keeping the canonical graph language-agnostic. Improve handling of unresolved symbols and report unsupported syntax and confidence limits explicitly.
3. **Verified test evidence.** Add optional, isolated test execution and coverage ingestion so CodeMind can distinguish static test recommendations from tests that were actually run and from observed runtime coverage. Repository execution would require explicit sandboxing and resource limits.
4. **Stronger investigation memory and collaboration.** Develop review feedback workflows, team-scoped reusable knowledge, approval/validation lifecycle controls, and auditable handoffs across longer investigations.
5. **Model and developer-tool integrations.** Add configurable reasoning-model providers and explore integrations such as MCP, code-hosting platforms, and CI workflows. Regardless of model provider, canonical code facts, authorization, and evidence remain controlled by the runtime and Rust engines.
6. **Production readiness.** Strengthen multi-user access control, operational observability, resource budgets, durable recovery, and deployment workflows as real usage requirements become clearer.

These are roadmap directions, not claims that the features are present in the hackathon build. Their order and scope will be decided after evaluating the MVP, user feedback, and implementation constraints. FalkorDB remains the graph database for the current project scope; a selectable multi-database layer is not part of this roadmap commitment.

---

## AI-Assistance Disclosure

AI agents are used extensively for CodeMind's implementation and testing work, including writing implementation code, creating and refining tests, and helping diagnose test failures. AI is the development tool used to help build the project; it is **not the project decision-maker**.

The project author makes the decisions about requirements, architecture, scope, priorities, trade-offs, and acceptance criteria. The author remains responsible for reviewing the implementation, verifying test results and acceptance gates, and ensuring that the final submission accurately describes what has actually been implemented and tested. This disclosure should remain consistent with the hackathon's rules.

---

## Status

CodeMind is being built and verified during the official Graph Hacks implementation window.

This README describes the intended product and architecture. A feature is considered complete only after its required tests and acceptance gate pass. Planned behavior is never treated as implemented behavior.
