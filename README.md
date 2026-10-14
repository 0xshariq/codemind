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
├── .gitignore
│
├── apps/
│   └── web/
│       ├── package.json
│       ├── pnpm-workspace.yaml
│       ├── playwright.config.ts
│       ├── .env.example
│       ├── docker-compose.yml
│       ├── next.config.ts
│       ├── tsconfig.json
│       ├── vitest.config.ts
│       └── src/
│           ├── app/
│           │   ├── layout.tsx
│           │   ├── page.tsx
│           │   ├── globals.css
│           │   ├── repositories/
│           │   │   ├── page.tsx
│           │   │   ├── new/
│           │   │   │   └── page.tsx
│           │   │   └── [repositoryId]/
│           │   │       └── page.tsx
│           │   ├── analysis/
│           │   │   └── [analysisId]/
│           │   │       └── page.tsx
│           │   ├── settings/
│           │   │   └── page.tsx
│           │   ├── help/
│           │   │   └── page.tsx
│           │   └── api/
│           │       ├── repositories/
│           │       │   ├── route.ts
│           │       │   └── [repositoryId]/
│           │       │       └── route.ts
│           │       ├── agent/
│           │       │   └── route.ts
│           │       ├── conversations/
│           │       │   ├── route.ts                 # List/create persisted conversation sessions
│           │       │   ├── search/
│           │       │   │   └── route.ts             # Search persisted conversation history
│           │       │   └── [conversationId]/
│           │       │       └── route.ts             # Load a session and its turns
│           │       ├── analysis/
│           │       │   └── [analysisId]/
│           │       │       └── route.ts
│           │       ├── operations/
│           │       │   └── [operationId]/
│           │       │       ├── route.ts
│           │       │       └── events/
│           │       │           └── route.ts
│           │       └── memory/
│           │           └── route.ts
│           │
│           ├── components/
│           │   ├── ui/                           # Shared design-system primitives
│           │   │   ├── button.tsx
│           │   │   ├── icon-button.tsx
│           │   │   ├── input.tsx
│           │   │   ├── textarea.tsx
│           │   │   ├── select.tsx
│           │   │   ├── badge.tsx
│           │   │   ├── chip.tsx
│           │   │   ├── card.tsx
│           │   │   ├── metric-card.tsx
│           │   │   ├── tabs.tsx
│           │   │   ├── accordion.tsx
│           │   │   ├── tooltip.tsx
│           │   │   ├── modal.tsx
│           │   │   ├── side-panel.tsx
│           │   │   ├── alert.tsx
│           │   │   ├── skeleton.tsx
│           │   │   ├── code-block.tsx
│           │   │   ├── timeline.tsx
│           │   │   ├── status-indicator.tsx
│           │   │   ├── data-table.tsx
│           │   │   └── result-list.tsx
│           │   ├── shell/
│           │   │   ├── app-shell.tsx
│           │   │   └── sidebar.tsx
│           │   ├── chat/
│           │   │   ├── chat-panel.tsx
│           │   │   ├── conversation-list.tsx
│           │   │   ├── conversation-search-modal.tsx
│           │   │   ├── message-list.tsx
│           │   │   ├── message-renderer.tsx
│           │   │   └── chat-composer.tsx
│           │   ├── execution/
│           │   │   ├── execution-timeline.tsx
│           │   │   ├── engine-accordion.tsx
│           │   │   └── operation-status.tsx
│           │   ├── analysis/
│           │   │   ├── summary-panel.tsx
│           │   │   ├── impact-card.tsx
│           │   │   ├── risk-card.tsx
│           │   │   ├── affected-files-panel.tsx
│           │   │   ├── test-impact-panel.tsx
│           │   │   └── recommendations-panel.tsx
│           │   ├── graph/
│           │   │   ├── dependency-graph.tsx
│           │   │   └── graph-path-panel.tsx
│           │   ├── evidence/
│           │   │   └── evidence-panel.tsx
│           │   ├── memory/
│           │   │   ├── memory-inspector.tsx
│           │   │   └── handoff-status.tsx
│           │   ├── repository/
│           │   │   ├── repository-card.tsx
│           │   │   ├── repository-list.tsx
│           │   │   └── repository-setup-form.tsx
│           │   ├── settings/
│           │   │   └── settings-navigation.tsx
│           │   └── common/
│           │       ├── error-state.tsx
│           │       ├── empty-state.tsx
│           │       ├── loading-state.tsx
│           │       └── completeness-notice.tsx
│           │
│           ├── tests/
│           │     ├── e2e/
│           │     │   ├── track01.spec.ts
│           │     │   ├── track02.spec.ts
│           │     │   └── reconnect-recovery.spec.ts
│           │     ├── language-adapters/
│           │     │   ├── rust-adapter.spec.ts
│           │     │   ├── typescript-adapter.spec.ts
│           │     │   └── python-adapter.spec.ts
│           │     ├── graph-conformance/
│           │     │   ├── canonical-model.spec.ts
│           │     │   ├── relationship-semantics.spec.ts
│           │     │   └── snapshot-isolation.spec.ts
│           │     └── memory/
│           │         ├── persistence.spec.ts
│           │         ├── handoff.spec.ts
│           │         └── scope-isolation.spec.ts
│           │
│           └── lib/
│               ├── shared/
│               │   ├── types.ts
│               │   └── status.ts
│               └── server/
│                   ├── agent/
│                   │   ├── runtime.ts
│                   │   ├── provider.ts
│                   │   ├── prompts.ts
│                   │   └── tool-loop.ts
│                   ├── tools/
│                   │   ├── registry.ts
│                   │   ├── catalog.ts
│                   │   ├── schemas.ts
│                   │   ├── executor.ts
│                   │   ├── guardrails.ts
│                   │   └── implementations/
│                   │       ├── get-repository-context.ts
│                   │       ├── search-code.ts
│                   │       ├── get-symbol.ts
│                   │       ├── query-code-relationships.ts
│                   │       ├── get-historical-changes.ts
│                   │       ├── get-test-intelligence.ts
│                   │       ├── analyze-change.ts
│                   │       ├── get-impact.ts
│                   │       ├── get-risk.ts
│                   │       ├── get-test-impact.ts
│                   │       └── get-evidence.ts
│                   ├── workflows/
│                   │   ├── prepare-repository.ts
│                   │   ├── investigate-change.ts
│                   │   └── finalize-review.ts
│                   ├── conversations/
│                   │   └── service.ts               # Session/turn orchestration via Rust gRPC
│                   ├── memory/
│                   │   ├── service.ts
│                   │   ├── policy.ts
│                   │   ├── retrieval.ts
│                   │   └── freshness.ts
│                   ├── handoffs/
│                   │   ├── service.ts
│                   │   └── schema.ts
│                   ├── operations/
│                   │   ├── service.ts
│                   │   ├── state.ts
│                   │   ├── events.ts
│                   │   └── sse.ts
│                   ├── results/
│                   │   ├── canonical-json.ts
│                   │   ├── validate.ts
│                   │   ├── llm-projection.ts
│                   │   ├── llm-encoding.ts
│                   │   ├── llm-encoding.test.ts
│                   │   └── ui-projection.ts
│                   ├── rust-client/
│                   │   ├── client.ts
│                   │   ├── progress-stream.ts
│                   │   └── generated/       # Generated; never hand-edit
│                   ├── auth/
│                   │   ├── context.ts
│                   │   └── authorize.ts
│                   └── config/
│                       └── env.ts
│
├── proto/
│     └── v1/
│         ├── common.proto
│         ├── repository.proto
│         ├── intelligence.proto
│         ├── memory.proto
│         └── operation.proto
│
├── engine/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── src/
│   │   ├── main.rs
│   │   ├── lib.rs
│   │   ├── config.rs
│   │   ├── transport/
│   │   │   ├── mod.rs
│   │   │   ├── server.rs
│   │   │   ├── progress.rs
│   │   │   └── conversions.rs
│   │   ├── contract/
│   │   │   ├── mod.rs
│   │   │   ├── context.rs
│   │   │   ├── repository.rs
│   │   │   ├── intelligence.rs
│   │   │   ├── memory.rs
│   │   │   ├── operation.rs
│   │   │   ├── result.rs
│   │   │   ├── events.rs
│   │   │   └── error.rs
│   │   ├── source/
│   │   │   ├── mod.rs
│   │   │   ├── provider.rs
│   │   │   ├── snapshot.rs
│   │   │   ├── manifest.rs
│   │   │   ├── language.rs
│   │   │   ├── limits.rs
│   │   │   └── tree_sitter.rs
│   │   ├── engines/
│   │   │   ├── mod.rs
│   │   │   ├── pipeline.rs
│   │   │   ├── coie/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── engine.rs
│   │   │   │   ├── model.rs
│   │   │   │   ├── graph_builder.rs
│   │   │   │   ├── extraction/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── normalize.rs
│   │   │   │   │   ├── validate.rs
│   │   │   │   │   ├── resolve.rs
│   │   │   │   │   └── evidence.rs
│   │   │   │   ├── languages/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── rust.rs
│   │   │   │   │   ├── javascript.rs
│   │   │   │   │   ├── typescript.rs
│   │   │   │   │   └── python.rs
│   │   │   │   └── queries/
│   │   │   │       ├── mod.rs
│   │   │   │       ├── symbols.rs
│   │   │   │       ├── search.rs
│   │   │   │       ├── relationships.rs
│   │   │   │       ├── paths.rs
│   │   │   │       └── history.rs
│   │   │   ├── tie/
│   │   │   │   ├── mod.rs
│   │   │   │   ├── engine.rs
│   │   │   │   ├── model.rs
│   │   │   │   ├── graph_builder.rs
│   │   │   │   ├── discovery/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── rust_tests.rs
│   │   │   │   │   ├── javascript_tests.rs
│   │   │   │   │   ├── typescript_tests.rs
│   │   │   │   │   └── python_tests.rs
│   │   │   │   ├── mapping/
│   │   │   │   │   ├── mod.rs
│   │   │   │   │   ├── target_mapping.rs
│   │   │   │   │   └── evidence.rs
│   │   │   │   └── queries/
│   │   │   │       ├── mod.rs
│   │   │   │       └── tests.rs
│   │   │   └── chie/
│   │   │       ├── mod.rs
│   │   │       ├── engine.rs
│   │   │       ├── model.rs
│   │   │       ├── change_set.rs
│   │   │       ├── graph_builder.rs
│   │   │       ├── impact/
│   │   │       │   ├── mod.rs
│   │   │       │   ├── traversal.rs
│   │   │       │   └── aggregation.rs
│   │   │       ├── risk/
│   │   │       │   ├── mod.rs
│   │   │       │   ├── factors.rs
│   │   │       │   └── assessment.rs
│   │   │       ├── test_impact.rs
│   │   │       ├── evidence.rs
│   │   │       └── queries.rs
│   │   ├── graph_storage/
│   │   │   ├── mod.rs
│   │   │   ├── port.rs
│   │   │   └── falkordb/
│   │   │       ├── mod.rs
│   │   │       ├── client.rs
│   │   │       ├── schema.rs
│   │   │       ├── read.rs
│   │   │       ├── write.rs
│   │   │       └── mapping.rs
│   │   ├── memory/
│   │   │   ├── mod.rs
│   │   │   ├── model.rs
│   │   │   ├── service.rs
│   │   │   ├── scope.rs
│   │   │   ├── validation.rs
│   │   │   ├── repository.rs
│   │   │   └── handoffs.rs
│   │   └── lifecycle/
│   │       ├── mod.rs
│   │       ├── operation.rs
│   │       ├── progress.rs
│   │       ├── events.rs
│   │       └── recovery.rs
│   └── tests/
│       ├── canonical_graph.rs
│       ├── language_adapters.rs
│       ├── falkordb_integration.rs
│       ├── grpc_contract.rs
│       ├── chie_impact.rs
│       ├── tie_discovery.rs
│       ├── memory_persistence.rs
│       ├── handoff_lifecycle.rs
│       └── operation_recovery.rs
│
│
├── docs/
│   ├── architecture.md
│   ├── testing.md
│   ├── design-system.md
│   ├── demo.md
│   ├── implementation-plan.md
│   └── design/                         # Approved reference images, using their real filenames
│       └── README.md                   # Inventory and screen/state mapping for those images
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
